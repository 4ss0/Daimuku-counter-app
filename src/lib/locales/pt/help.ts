import type { HelpTexts } from '../types';

export default {
  voice: [
    'Antes de começar: sua voz',
    'Na primeira vez, o app abre “Sua voz”. Escolha uma velocidade (Lento, Médio ou Rápido), defina quantos daimoku vai recitar, toque em Gravar, recite exatamente essa quantidade de daimoku e toque em Concluir. Depois toque em Adicionar: o app aprende com essa gravação.\n\nFaça pelo menos uma gravação em cada velocidade. Você pode adicionar mais quando quiser: se a contagem erra com frequência, mais uma gravação na velocidade em que você costuma recitar ajuda muito.',
  ],
  count: [
    'Contar os daimoku',
    'Na tela Contar toque em Iniciar e recite. O número sobe assim que o app ouve o “kyo” final de cada daimoku; o ponto ao lado do status acende quando ele ouve você.\n\nQuando terminar, toque em Parar: a sessão é salva. Se o app pulou alguns ou contou a mais, corrija o número com − e +: as estatísticas usam o valor corrigido. Você também pode excluir a sessão.',
  ],
  coins: [
    'Moedas: os daimoku recuperados',
    'Além do reconhecimento imediato, a cada dois segundos o app escuta de novo, com mais atenção, o último minuto. Se encontrar daimoku que escaparam (cobertos pelo sino, pelo juzu ou por um ruído), ele os adiciona: você os vê chegar como moedas voando para o contador.',
  ],
  screen: [
    'Com a tela desligada',
    'Você pode desligar a tela ou usar outros apps: a contagem continua e uma notificação mostra o número atual. Quando você volta ao app, o contador já mostra o número certo.\n\nEm Ajustes você pode escolher manter a tela ligada durante a contagem. Mesmo que vá para outra tela (Estatísticas, Dicionário…), a sessão continua: a barra superior mostra a contagem; toque nela para voltar ao contador.',
  ],
  tips: [
    'Dicas para uma contagem precisa',
    '• Mantenha o celular perto de você, sem cobrir o microfone.\n• Diga cada daimoku por inteiro, até o “kyo”: uma frase interrompida antes não é contada.\n• Ruídos fortes e contínuos perto do celular (o sino, o juzu esfregado) podem fazer alguns daimoku escaparem: as verificações recuperam a maioria.\n• A contagem funciona em grupo, mas é mais precisa quando a voz mais próxima do celular é a sua.\n• Se a contagem erra com frequência, adicione gravações da sua voz.',
  ],
  stats: [
    'Estatísticas, meta e compartilhar',
    'Em Estatísticas você vê seus daimoku por dia, semana, mês e ano, sua sequência e sua média: toque em um ponto do gráfico para ver os detalhes. Em Ajustes escolha qual período aparece primeiro. Defina uma meta diária: o anel da tela Contar se enche até você alcançá-la.\n\nRecitou sem o app, por exemplo numa reunião? Em “Sessões” escolha o dia e a sessão (manhã ou noite) e digite quantos daimoku recitou. Com o ícone de compartilhar você pode compartilhar uma sessão, um período ou uma meta alcançada.',
  ],
  goals: [
    'Metas e lembretes',
    'Em Ajustes → Metas de daimoku escolha sua meta diária e, se quiser, uma semanal, uma mensal e uma anual. A meta diária é dividida em duas sessões: a manhã (até as 14h) e a noite; a tela Contar mostra quanto você já fez de cada uma.\n\nO contador continua a sessão atual, a partir dos daimoku já recitados. Para continuar a outra (por exemplo, o daimoku da manhã feito à tarde), toque nela antes de tocar em Iniciar. Cada sessão salva fica na manhã ou na noite que você escolheu.\n\nNo menu, Lembretes envia uma notificação no horário que você escolher para a sessão da manhã e para a da noite.',
  ],
  intents: [
    'Seus objetivos',
    'No menu, Objetivos é onde você escreve pelo que recita: um ou mais objetivos, com suas próprias palavras. Embaixo de cada um, mova o controle para mostrar quanto você já avançou.\n\nQuando chegar a hora, toque em Fechar e escolha o motivo: o objetivo foi alcançado, ou você percebeu que havia um caminho melhor. Os objetivos fechados ficam no fim da página: você pode relê-los, reabri-los ou apagá-los.',
  ],
  badges: [
    'Conquistas e o dia perdoado',
    'Você ganha uma conquista aos 100, 1.000, 10.000, 1 milhão, 5 e 10 milhões de daimoku, e pelos dias seguidos de recitação: uma semana, um mês, seis meses e depois 1, 2, 5, 10, 15 e 20 anos. Você encontra todas no menu, em Conquistas, junto com o dia perdoado.\n\nSe num dia você não recitou, uma vez por semana pode perdoá-lo com o botão em Estatísticas: sua sequência de dias seguidos não se quebra.',
  ],
  bell: [
    'Sino e fundo',
    'No menu, Sino permite escolher entre três sinos, do mais agudo ao mais grave. Se você ativar o botão, aparece um sino ao lado de Iniciar na tela Contar: tocá-lo é como bater nele. Você pode tocá-lo durante a recitação: o contador não o ouve, porque retira as notas desse sino do microfone enquanto ele soa.\n\nEm Ajustes → Fundo escolha o lótus ou uma das cinco ilustrações (Sutra de Lótus, Buda, templo, natureza, cerejeiras em flor), cada uma clara ou escura conforme o tema, ou uma imagem sua.',
  ],
  backup: [
    'Backup e troca de celular',
    'Em Ajustes, em “Seus dados”, ative o Salvamento automático: na primeira vez você escolhe onde guardar o arquivo (no celular ou no Drive) e depois o app salva tudo sozinho a cada sessão, a cada mudança nas suas gravações de voz e a cada conquista. Num celular novo, ou se reinstalar o app, escolha “Carregar um salvamento” na primeira abertura e selecione esse arquivo.\n\nEm Ajustes, em “Seus dados”, crie um backup: ele contém suas sessões, metas, ajustes e gravações de voz. Guarde-o fora do celular (Drive, e-mail) e no celular novo use “Restaurar de um backup”. Você também pode exportar suas sessões para um arquivo CSV.',
  ],
  menu: [
    'O menu',
    'Toque em ☰ no canto superior esquerdo, ou deslize para a direita a partir da metade esquerda da tela, para abrir o menu: contador, objetivos, Gongyo, conquistas, lembretes, sino, dicionário de conceitos budistas, doações, este guia e privacidade. Para fechá-lo deslize para a esquerda ou toque fora.\n\nNo Gongyo o texto sobe como os créditos de um filme e as letras se colorem à medida que você recita, como num karaokê: no meio as duas linhas a ler, em cima as duas já recitadas, embaixo as seguintes. Ajuste a velocidade com o controle: enquanto o move você vê uma prévia. Em “Tradução” você encontra os dois capítulos linha por linha, no idioma que escolher.\n\nSempre que abre o app você encontra a frase do dia, dos escritos de Nichiren Daishonin, do Sutra de Lótus ou dos presidentes da Soka Gakkai. As setas (ou deslizar) mostram as frases dos outros dias; “Entrar” leva você ao app.',
  ],
  trouble: [
    'Se algo der errado',
    '• “O app não pode usar o microfone”: permita o microfone nas configurações do Android (Apps → Daimoku Counter → Permissões).\n• Nenhuma notificação com a tela desligada: permita as notificações do app.\n• A contagem para com a tela desligada: alguns celulares fecham apps para economizar bateria; tire o Daimoku Counter da otimização de bateria.\n• A contagem erra: depois de Parar toque em “Salvar o áudio para relatar um problema” e envie o arquivo ao desenvolvedor (o contato está na política de privacidade completa, na página Privacidade).',
  ],
} satisfies HelpTexts;
