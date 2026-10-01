import ctypes, wave, struct, espeakng_loader as el, sys
lib=ctypes.CDLL(el.get_library_path())
sr=lib.espeak_Initialize(2, 500, el.get_data_path().encode(), 0)
buf=[]
CB=ctypes.CFUNCTYPE(ctypes.c_int, ctypes.POINTER(ctypes.c_short), ctypes.c_int, ctypes.c_void_p)
def cb(w,n,e):
    if n>0: buf.extend(w[:n])
    return 0
cbf=CB(cb); lib.espeak_SetSynthCallback(cbf)
def say(voice, text, rate, pitch, out):
    buf.clear()
    lib.espeak_SetVoiceByName(voice.encode())
    lib.espeak_SetParameter(1, rate, 0); lib.espeak_SetParameter(3, pitch, 0)
    t=text.encode('utf-8')
    lib.espeak_Synth(t, len(t)+1, 0, 0, 0, 1, None, None)
    lib.espeak_Synchronize()
    with wave.open(out,'wb') as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr); w.writeframes(struct.pack('<%dh'%len(buf), *buf))
    print(out, len(buf)/sr)
it="Oggi siamo andati al mercato a comprare la frutta e la verdura. Poi abbiamo incontrato Marco, che ci ha raccontato del suo viaggio a Kyoto e a Nara. Nel pomeriggio ho letto un libro sulla storia del Giappone, sulle montagne, sui templi e sui giardini. La sera abbiamo mangiato insieme e parlato di tante cose: del lavoro, della famiglia, dei progetti per l'estate. Domani mattina mi sveglio presto, faccio colazione, prendo il treno delle sette e vado in ufficio. Ricordati di chiamare la nonna e di comprare il pane."
en="Good morning everyone. Today we are going to talk about the meeting schedule, the new project and the budget for next year. Please remember to send your reports by Friday. After lunch we will review the design, and then we can discuss any remaining questions. My name is Anna and I live in a small town near the mountains, where the rivers are clean and the air is fresh."
ja="今日はとても良い天気ですね。朝ごはんを食べてから、駅まで歩いて、電車で京都へ行きました。お寺をたくさん見て、美味しいお茶を飲みました。明日は友達と会う予定です。仕事の話や家族の話をしました。"
tricky="Nami, mio, oro, renne, kyoto. Namo, miao, orecchio, rendere, chiodo. Nam myo ho ren ge. Myoho renge. Ren ge kyo ren ge kyo. Kyo ho myo nam."
for i,(v,t) in enumerate([("it",it),("en",en),("ja",ja),("it",tricky)]):
    for j,(rate,pitch) in enumerate([(150,40),(210,70),(120,25)]):
        say(v, t, rate, pitch, f"tts_{v}{i}_{j}.wav")
# espeak chanting daimoku (robotic positive, informational)
say("ja", "南無妙法蓮華経、"*10, 260, 50, "tts_daimoku10.wav")
