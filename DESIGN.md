# DESIGN.md

## Overview

Uma linguagem de design institucional para mini-apps locais construidas sobre esta
workspace Rust.

Este design e otimizado para:

- sessoes longas de trabalho
- fluxos repetiveis
- navegacao previsivel
- superficies densas de informacao
- separacao clara entre shell e conteudo util

Deve transmitir:

- calma
- oficialidade
- estrutura
- legibilidade rapida
- sobriedade operacional

Nao deve transmitir:

- interface ludica
- visual "startup SaaS"
- excesso de cartoes
- decoracao gratuita
- experimentacao visual ad-hoc por app

A referencia visual para shell e hierarquia e a imagem
`.artifacts/IMG_2582.PNG`: barra superior institucional, rail esquerdo de
navegacao, header de workspace, rail horizontal de fluxo/tarefas, rail direito
de utilitarios e tela central maximizada para conteudo.

Neste projeto, a estrutura da workspace mantem-se:

- `crates/support-*` fornecem capacidades headless e transversais
- `apps/*` compoem hosts concretos
- futuras UIs graficas devem respeitar a shell institucional comum em vez de
  inventarem uma linguagem visual por mini-app

CLI e TUI continuam interfaces de primeira classe. Nao replicam a shell visual
da imagem, mas devem preservar a mesma hierarquia de tarefas, estados,
terminologia e contexto operacional.

O modo escuro da referencia e valido, mas o modo claro deve preservar:

- a mesma hierarquia
- a mesma densidade
- a mesma estrutura de shell
- as mesmas regras de enfase
- o mesmo uso contido do acento cromatico

---

## Core Layout

A shell institucional de referencia organiza-se em cinco zonas persistentes:

1. **Barra institucional superior**
   - contem marca/shell, seletor de contexto, controlos de tema e identidade do utilizador
   - deve ser compacta, estavel e discreta
   - nunca deve parecer header promocional

2. **Rail de navegacao esquerdo**
   - navegacao principal por areas, modulos ou familias de mini-apps
   - suporta grupos colapsaveis e niveis hierarquicos
   - icones ajudam, mas os rotulos continuam a ser a fonte primaria de clareza

3. **Header de workspace**
   - mostra workflow, grupo, entidade corrente e pagina/subarea ativa
   - breadcrumbs e contexto devem ser sempre explicitos

4. **Rail horizontal de fluxo/tarefas**
   - serve para etapas de workflow, tabs ou sub-views principais
   - e uma zona critica e deve parecer precisa, nao decorativa

5. **Rail direito de utilitarios**
   - favoritos, historico, acoes rapidas, configuracoes e ferramentas contextuais
   - deve ser compacto e claramente secundario face ao workspace principal

A tela central deve ser maximizada para o conteudo efetivo da mini-app.
Evitar wrappers desnecessarios, paineis pesados ou grelhas de cartoes sem
justificacao semantica.

Para hosts sem GUI:

- CLI e TUI devem mapear esta estrutura para uma hierarquia equivalente
- comandos principais equivalem a navegacao estrutural
- subcomandos ou modos equivalem a rails locais de workflow
- contexto atual deve ser sempre visivel

---

## Visual Tone

- Usar superficies mate, nao brilhantes.
- Usar separadores finos e fronteiras discretas para criar estrutura.
- Usar profundidade com moderacao.
- Favorecer estabilidade visual sobre expressividade.

A shell deve parecer desenhada para operacao diaria, nao para demonstracao.

---

## Color System

A paleta e maioritariamente neutra, com uma familia de acento controlada.

### Neutrals

Os neutros dominam:

- fundos de shell
- superficies do workspace
- fronteiras e separadores
- labels mutedos
- tabs inativas
- paineis utilitarios

Referencia dark:

- **Canvas / app background**: `#121212`
- **Shell surface**: `#171717` a `#1E1E1E`
- **Raised surface**: `#232323` a `#2A2A2A`
- **Border / divider**: `rgba(255,255,255,0.14)` a `rgba(255,255,255,0.20)`
- **Primary text**: `#F5F5F5`
- **Secondary text**: `#C7C7C7`
- **Muted text**: `#9A9A9A`

Referencia light:

- **Canvas / app background**: `#F5F5F3`
- **Shell surface**: `#FFFFFF`
- **Raised surface**: `#F0F0ED`
- **Border / divider**: `rgba(0,0,0,0.10)` a `rgba(0,0,0,0.16)`
- **Primary text**: `#161616`
- **Secondary text**: `#404040`
- **Muted text**: `#6A6A6A`

### Accent

O acento deve seguir a familia quente da referencia, mais proxima de
ambar/dourado do que do azul SaaS generico.

Recomendado:

- **Accent / active emphasis**: `#D6A126`
- **Accent hover**: `#E0AF39`
- **Accent subdued fill**: `rgba(214,161,38,0.14)`

Regras de uso:

- usar para etapa ativa, selecao corrente e sinais de workflow
- preferir underline, outline, icone enfatizado ou pill compacto
- evitar grandes superficies pintadas com acento
- em cada viewport deve existir normalmente um unico foco dominante

### Semantic States

- **Error**: `#C94A3F`
- **Success**: `#4E9A51`
- **Info**: `#3C82D6`
- **Warning**: derivado da familia quente, mas distinto da selecao ativa

As cores semanticas nao devem esmagar a shell neutra.

---

## Typography

- **Primary font (GUI futura)**: Inter ou Source Sans 3
- **Fallbacks**: system sans-serif stack
- **Terminal parity**: em CLI/TUI, privilegiar largura previsivel, labels curtas e alinhamento claro

A tipografia deve parecer operacional e compacta.

Escala recomendada:

- titulo de workspace: 28-34px, semibold
- titulo de secao: 20-24px, semibold
- label de navegacao: 15-16px, medium a semibold
- body text: 14-15px, regular
- suporte / metadata: 12-13px

Regras:

- criar hierarquia com peso e espacamento, nao com demasiados tamanhos
- manter labels curtas e explicitas
- evitar display typography exagerada
- garantir legibilidade em shells densas e paineis laterais

---

## Spacing and Density

Esta workspace visa mini-apps operacionais, nao landing pages arejadas.

- Preferir ritmo de 8px.
- Usar espacamento vertical compacto em navegacao e tool rails.
- Manter paineis compactos mas respiraveis.
- Reservar o maior espaco livre para o conteudo central, nao para o chrome.

Principios de densidade:

- itens de navegacao podem ser compactos
- targets continuam a ter de ser confortavelmente clicaveis
- tabs e rails horizontais devem suportar muitos itens sem ruido
- formularios e listas devem alinhar de forma rigorosa
- tabelas densas sao aceitaveis se a leitura continuar forte

---

## Elevation and Surfaces

Evitar sombras pesadas.

Criar profundidade atraves de:

- contraste entre superficies
- agrupamento interno
- fronteiras
- overlays subtis

Elevacao permitida:

- dropdowns
- dialogs
- popovers
- tooltips

Mesmo nesses casos, as sombras devem ser suaves e controladas.

---

## Components

O projeto atual ainda e headless-first, por isso este documento define o
comportamento visual para futuras shells GUI e a disciplina de composicao para
hosts em geral.

Regras de composicao:

- componentes de shell devem ser reutilizaveis entre mini-apps
- a linguagem visual nao deve nascer dentro de cada `apps/*`
- `crates/support-*` nao devem carregar dependencias de UI
- a UI deve compor-se por cima dos contratos e runtimes existentes, nao contorna-los

Equivalente para interfaces de terminal:

- nao inventar gramatica de comandos diferente por mini-app sem necessidade forte
- manter naming consistente com o modelo documental e operacional do workspace
- preservar contexto, fluxo e estados de forma clara

### Buttons

- Radius: `8px` a `10px`
- botoes primarios devem ser usados com moderacao
- acoes secundarias e terciarias devem dominar o uso rotineiro
- preferir outline, ghost ou fills subtis dentro da shell

Regra:

- reservar a maior enfase para a unica acao principal de cada area local

### Inputs and Selects

- altura compacta
- fronteira clara
- contraste discreto de superficie
- focus ring visivel
- bom alinhamento em barras superiores, filtros e toolbars

Seletores de contexto podem viver dentro de capsulas arredondadas e discretas,
como na referencia.

### Tabs and Workflow Rails

Este e um dos padroes definidores da referencia.

- tabs horizontais devem ser text-first com icones opcionais
- o item ativo deve usar underline espesso ou contorno forte
- o underline pode ser mais espesso do que em tabs web genericas
- itens inativos devem permanecer legiveis mas discretos

Evitar:

- pill tabs espalhadas por todo o interface
- segmented controls muito coloridos
- barras de tabs sobredimensionadas

### Navigation Rail

- o rail esquerdo deve parecer estrutura persistente e institucional
- niveis aninhados devem ser claros por indentacao e agrupamento
- grupos expandidos devem manter leitura limpa
- o item corrente pode usar bordo/acento lateral ou highlight subtil de linha

### Utility Rail

- o rail direito e icon-first
- manter alinhamento vertical rigoroso
- tooltips devem ser utilitarios e diretos

### Panels and Cards

- preferir paineis planos com fronteira em vez de "cards" soltos
- usar cards apenas quando o agrupamento for semanticamente util
- evitar dashboards de mosaicos como default

### Tables and Lists

- alinhamento forte
- estados de linha claros
- headers sticky quando fizer sentido
- selecao evidente
- densidade alta e aceitavel, desde que a leitura se mantenha

---

## Interaction Patterns

- Preferir progressive disclosure a fluxos cheios de modais.
- Manter o utilizador ancorado na shell.
- Preservar contexto visivel ao mudar de etapa, lista ou detalhe.
- Tornar a localizacao atual obvia em permanencia.

Transicoes:

- subtis
- rapidas
- intencionais

Evitar:

- teatro de animacao
- transicoes surpresa
- acoes criticas escondidas
- arvores profundas que apagam o contexto atual

Para hosts CLI/TUI:

- o contexto atual deve aparecer sempre no output
- comandos destrutivos devem ser claros e deliberados
- listagens, detalhe e acao devem seguir um modelo estavel entre apps

---

## Accessibility

A acessibilidade e estrutural, nao acabamento.

- contraste minimo WCAG AA
- foco de teclado visivel em todas as zonas interativas
- nao depender so da cor para estado ativo ou erro
- manter tamanhos de target confortaveis mesmo em layout denso
- tooltips e controlos utilitarios devem ser acessiveis por teclado
- dark e light mode devem preservar a mesma hierarquia informativa

Cuidado especial para:

- operadores em sessoes longas
- utilizadores com baixa visao
- alternancia frequente entre teclado e rato
- interfaces de terminal usadas em contexto de suporte ou operacao

---

## Do's and Don'ts

### Do

- preservar a estrutura shell-first
- usar neutros como linguagem visual dominante
- usar o acento quente com moderacao
- maximizar o espaco do canvas de trabalho
- tornar a hierarquia explicita pelo layout
- fazer com que mini-apps diferentes parecam parte da mesma familia
- manter paridade conceptual entre GUI futura e hosts CLI/TUI

### Don't

- transformar a UI num dashboard SaaS generico
- trocar a shell por cartoes soltos em canvas vazio
- usar azul brilhante como destaque por defeito
- abusar de sombras, vidro ou brilho
- introduzir animacao decorativa
- permitir que cada app crie a sua propria linguagem de shell

---

## Design Intent for Agents

Ao gerar UI, fluxos ou hosts operacionais, pensar em:

- shell institucional
- densidade desktop
- hierarquia de navegacao explicita
- enfase quente e contida
- conteudo operacional legivel
- paridade entre GUI, CLI e TUI quando aplicavel

Decisoes por defeito:

- escolher estrutura antes de novidade
- escolher densidade calma antes de vazio artificial
- escolher hierarquia por superficies e fronteiras antes de sombras
- escolher um unico foco de acento antes de varios highlights
- escolher componentes/shared shell antes de design local por app
- escolher semantica de tarefas partilhada antes de gramatica ad-hoc por host

Se houver duvida, desenhar a mini-app como parte de uma shell institucional
usada diariamente por operadores que valorizam previsibilidade, rapidez e
legibilidade.
