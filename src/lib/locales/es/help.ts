import type { HelpTexts } from '../types';

export default {
  voice: [
    'Antes de empezar: tu voz',
    'La primera vez, la app abre «Tu voz». Elige una velocidad (Lento, Medio o Rápido), indica cuántos daimoku vas a entonar, pulsa Grabar, entona exactamente esa cantidad de daimoku y pulsa Listo. Luego toca Añadir: la app aprende de esa grabación.\n\nHaz al menos una grabación de cada velocidad. Puedes añadir más cuando quieras: si el conteo falla a menudo, una grabación más a la velocidad a la que sueles entonar ayuda mucho.',
  ],
  count: [
    'Contar los daimoku',
    'En la pantalla Contar pulsa Iniciar y entona. El número sube en cuanto la app oye el «kyo» final de cada daimoku; el punto junto al estado se enciende cuando te oye.\n\nCuando termines pulsa Detener: la sesión se guarda. Si la app se saltó algunos o añadió de más, corrige el número con − y +: las estadísticas usan el valor corregido. También puedes eliminar la sesión.',
  ],
  coins: [
    'Monedas: los daimoku recuperados',
    'Además del reconocimiento inmediato, cada dos segundos la app vuelve a escuchar, con más atención, el último minuto. Si encuentra daimoku que se le escaparon (tapados por la campana, el juzu o un ruido), los añade: los ves llegar como monedas que vuelan al contador.',
  ],
  screen: [
    'Con la pantalla apagada',
    'Puedes apagar la pantalla o usar otras apps: el conteo continúa y una notificación muestra el número actual. Cuando vuelves a la app, el contador ya muestra el número correcto.\n\nEn Ajustes puedes elegir mantener la pantalla encendida mientras cuenta. Aunque pases a otra pantalla (Estadísticas, Diccionario…), la sesión continúa: la barra superior muestra el conteo; tócala para volver al contador.',
  ],
  tips: [
    'Consejos para un conteo preciso',
    '• Ten el teléfono cerca de ti, sin tapar el micrófono.\n• Di cada daimoku completo, hasta «kyo»: una frase cortada antes no se cuenta.\n• Los ruidos fuertes y continuos cerca del teléfono (la campana, el juzu frotado) pueden hacer que se escapen algunos daimoku: las comprobaciones recuperan la mayoría.\n• El conteo funciona en grupo, pero es más preciso cuando la voz más cercana al teléfono es la tuya.\n• Si el conteo falla a menudo, añade grabaciones de tu voz.',
  ],
  stats: [
    'Estadísticas, meta y compartir',
    'En Estadísticas ves tus daimoku por día, semana, mes y año, tu racha y tu media: toca un punto del gráfico para ver los detalles. En Ajustes elige qué periodo se muestra primero. Fija una meta diaria: el anillo de la pantalla Contar se llena hasta alcanzarla.\n\n¿Entonaste sin la app, por ejemplo en una reunión? En «Sesiones» elige el día y la sesión (mañana o tarde) y escribe cuántos daimoku entonaste. Con el icono de compartir puedes compartir una sesión, un periodo o una meta alcanzada.',
  ],
  goals: [
    'Metas y recordatorios',
    'En Ajustes → Metas elige tu meta diaria y, si quieres, una semanal, una mensual y una anual. La meta diaria se divide en dos sesiones: la mañana (hasta las 14:00) y la tarde; la pantalla Contar muestra cuánto llevas de cada una.\n\nEl contador continúa la sesión actual, partiendo de los daimoku ya entonados. Para continuar la otra (por ejemplo, el daimoku de la mañana hecho por la tarde), tócala antes de pulsar Iniciar. Cada sesión guardada queda en la mañana o en la tarde que elegiste.\n\nEn el menú, Recordatorios te envía una notificación a la hora que elijas para la sesión de la mañana y para la de la tarde.',
  ],
  badges: [
    'Logros y el día perdonado',
    'Ganas un logro a los 100, 1.000, 10.000, 1 millón, 5 y 10 millones de daimoku, y por los días seguidos de entonación: una semana, un mes, seis meses y luego 1, 2, 5, 10, 15 y 20 años. Los encuentras todos en el menú, en Logros, junto con el día perdonado.\n\nSi un día no entonaste, una vez por semana puedes perdonarlo con el botón de Estadísticas: tu racha de días seguidos no se rompe.',
  ],
  bell: [
    'Campana y fondo',
    'En el menú, Campana te deja elegir entre tres campanas, de la más aguda a la más grave. Si activas su botón, aparece una campana junto a Iniciar en la pantalla Contar: tocarla es como golpearla. Puedes hacerla sonar mientras entonas: el contador no la oye, porque quita las notas de esa campana del micrófono mientras suena.\n\nEn Ajustes → Fondo elige el loto o una de las cinco ilustraciones (Sutra del loto, Buda, templo, naturaleza, cerezos en flor), cada una clara u oscura según el tema, o una imagen tuya.',
  ],
  backup: [
    'Copia de seguridad y cambio de teléfono',
    'En Ajustes, en «Tus datos», crea una copia de seguridad: contiene tus sesiones, metas, ajustes y grabaciones de voz. Guárdala fuera del teléfono (Drive, correo) y en el teléfono nuevo usa «Restaurar desde una copia». También puedes exportar tus sesiones a un archivo CSV.',
  ],
  menu: [
    'El menú',
    'Toca ☰ arriba a la izquierda, o desliza hacia la derecha desde la mitad izquierda de la pantalla, para abrir el menú: contador, Gongyo, logros, recordatorios, campana, diccionario de conceptos budistas, donaciones, esta guía y privacidad. Para cerrarlo desliza hacia la izquierda o toca fuera.\n\nEn Gongyo el texto sube como los créditos de una película y las letras se colorean a medida que recitas, como en un karaoke: en el centro las dos líneas que leer, arriba las dos ya recitadas, abajo las siguientes. Ajusta la velocidad con el control: mientras lo mueves ves una muestra. En «Traducción» encuentras los dos capítulos línea por línea, en el idioma que elijas.\n\nCada vez que abres la app encuentras la frase del día, de los escritos de Nichiren Daishonin, del Sutra del loto o de los presidentes de la Soka Gakkai. Las flechas (o deslizar) muestran las frases de otros días; «Entrar» te lleva a la app.',
  ],
  trouble: [
    'Si algo no funciona',
    '• «La app no puede usar el micrófono»: permite el micrófono en los ajustes de Android (Aplicaciones → Daimoku Counter → Permisos).\n• No aparece la notificación con la pantalla apagada: permite las notificaciones de la app.\n• El conteo se detiene con la pantalla apagada: algunos teléfonos cierran las apps para ahorrar batería; excluye Daimoku Counter de la optimización de batería.\n• El conteo falla: después de Detener toca «Guardar el audio para informar de un problema» y envía el archivo al desarrollador (el contacto está en la política de privacidad completa, en la página Privacidad).',
  ],
} satisfies HelpTexts;
