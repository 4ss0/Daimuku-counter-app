package com.daimuku.counter

import android.app.Activity
import com.android.billingclient.api.BillingClient
import com.android.billingclient.api.BillingClient.BillingResponseCode
import com.android.billingclient.api.BillingClientStateListener
import com.android.billingclient.api.BillingFlowParams
import com.android.billingclient.api.BillingResult
import com.android.billingclient.api.ConsumeParams
import com.android.billingclient.api.PendingPurchasesParams
import com.android.billingclient.api.ProductDetails
import com.android.billingclient.api.Purchase
import com.android.billingclient.api.PurchasesUpdatedListener
import com.android.billingclient.api.QueryProductDetailsParams
import com.android.billingclient.api.QueryPurchasesParams
import org.json.JSONArray
import org.json.JSONObject

/**
 * Donations through Google Play Billing. Each donation is a consumable
 * in-app product (created in the Play Console with the ids the web page
 * asks for): it is consumed right away, so it can be given again, and it
 * unlocks nothing.
 *
 * Results go back to the page through [emit] (event, ok, data):
 *   "products" -> JSON [{"id", "price"}] of the products Play knows;
 *   "donate"   -> "thanks" | "pending" | "cancelled" | "unavailable" | "error".
 */
class Donations(
  private val activity: Activity,
  private val emit: (event: String, ok: Boolean, data: String?) -> Unit,
) : PurchasesUpdatedListener {
  private val client: BillingClient = BillingClient.newBuilder(activity)
    .setListener(this)
    .enablePendingPurchases(PendingPurchasesParams.newBuilder().enableOneTimeProducts().build())
    .build()
  private val details = mutableMapOf<String, ProductDetails>()
  private val waiting = mutableListOf<(Boolean) -> Unit>()
  private var connecting = false

  /** Runs [then] once connected to Play (false: Play not available). */
  @Synchronized
  private fun connected(then: (Boolean) -> Unit) {
    if (client.isReady) {
      then(true)
      return
    }
    waiting.add(then)
    if (connecting) return
    connecting = true
    client.startConnection(object : BillingClientStateListener {
      override fun onBillingSetupFinished(result: BillingResult) {
        settle(result.responseCode == BillingResponseCode.OK)
      }

      override fun onBillingServiceDisconnected() {
        settle(false)
      }
    })
  }

  @Synchronized
  private fun settle(ok: Boolean) {
    connecting = false
    val all = waiting.toList()
    waiting.clear()
    all.forEach { it(ok) }
  }

  /** Prices of the given products, in the user's currency. */
  fun products(ids: List<String>) = connected { ok ->
    if (!ok || ids.isEmpty()) {
      emit("products", false, null)
      return@connected
    }
    consumeLeftovers()
    val params = QueryProductDetailsParams.newBuilder()
      .setProductList(ids.map {
        QueryProductDetailsParams.Product.newBuilder()
          .setProductId(it)
          .setProductType(BillingClient.ProductType.INAPP)
          .build()
      })
      .build()
    client.queryProductDetailsAsync(params) { result, found ->
      val list = found.productDetailsList
      val out = JSONArray()
      synchronized(details) {
        for (pd in list) {
          details[pd.productId] = pd
          out.put(JSONObject().put("id", pd.productId).put("price", pd.oneTimePurchaseOfferDetails?.formattedPrice ?: ""))
        }
      }
      emit("products", result.responseCode == BillingResponseCode.OK && list.isNotEmpty(), out.toString())
    }
  }

  /** Opens the Google Play payment sheet for one product. */
  fun donate(id: String) = connected { ok ->
    val pd = synchronized(details) { details[id] }
    if (!ok || pd == null) {
      emit("donate", false, "unavailable")
      return@connected
    }
    activity.runOnUiThread {
      val params = BillingFlowParams.newBuilder()
        .setProductDetailsParamsList(listOf(BillingFlowParams.ProductDetailsParams.newBuilder().setProductDetails(pd).build()))
        .build()
      val r = client.launchBillingFlow(activity, params)
      if (r.responseCode != BillingResponseCode.OK) emit("donate", false, "error")
    }
  }

  override fun onPurchasesUpdated(result: BillingResult, purchases: MutableList<Purchase>?) {
    when (result.responseCode) {
      BillingResponseCode.OK -> {
        var thanks = false
        var pending = false
        for (p in purchases.orEmpty()) {
          when (p.purchaseState) {
            Purchase.PurchaseState.PURCHASED -> {
              consume(p)
              thanks = true
            }
            Purchase.PurchaseState.PENDING -> pending = true
          }
        }
        when {
          thanks -> emit("donate", true, "thanks")
          pending -> emit("donate", true, "pending")
          else -> emit("donate", false, "error")
        }
      }
      BillingResponseCode.USER_CANCELED -> emit("donate", false, "cancelled")
      BillingResponseCode.ITEM_ALREADY_OWNED -> {
        // a previous donation was not consumed (app closed too early)
        consumeLeftovers()
        emit("donate", false, "error")
      }
      else -> emit("donate", false, "error")
    }
  }

  /** Consumes donations completed earlier (e.g. a pending payment done later). */
  private fun consumeLeftovers() {
    val params = QueryPurchasesParams.newBuilder().setProductType(BillingClient.ProductType.INAPP).build()
    client.queryPurchasesAsync(params) { result, purchases ->
      if (result.responseCode != BillingResponseCode.OK) return@queryPurchasesAsync
      purchases.filter { it.purchaseState == Purchase.PurchaseState.PURCHASED }.forEach { consume(it) }
    }
  }

  private fun consume(p: Purchase) {
    client.consumeAsync(ConsumeParams.newBuilder().setPurchaseToken(p.purchaseToken).build()) { _, _ -> }
  }

  fun end() {
    if (client.isReady) client.endConnection()
  }
}
