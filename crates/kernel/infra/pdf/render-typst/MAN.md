# Manual: render-typst

## Contrato publico

Com a feature `pdf` ativa:

- `pdf::compile_pdf(source, fonts_dir)` compila Typst para bytes PDF.
- `pdf::compile_with_fonts_timed(source, font_bytes)` devolve PDF e metricas.
- `pdf::compile_with_warm_fonts_timed(source, warm)` reutiliza `WarmFonts`.
- `pdf::compile_with_warm_fonts_and_files(source, warm, extra_files)` permite
  imagens/ficheiros inline.
- `pdf::WarmFonts` guarda fontes pre-parseadas para compilações repetidas.

Helpers leves de template vivem em `support-typst-template`.

## Como usar

O host ou pipeline escolhe este adapter quando precisa de rendering Typst real.
Crates de dominio devem depender dele por feature quando Typst for opcional.

## Invariantes e regras

- O crate e adapter concreto de infra, nao contrato de dominio.
- Rendering PDF depende da feature `pdf`.
- `WarmFonts` deve ser criado uma vez no arranque do host quando ha muitas
  compilacoes.
- Ficheiros adicionais sao servidos em memoria ao compilador Typst.

## Limitacoes atuais

- O adapter nao fornece sandbox documental formal nem autorizacao.
- Erros de compilacao sao devolvidos como texto tecnico.

## ToDo

- Definir um port comum de rendering PDF se surgirem mais backends.
- Melhorar diagnosticos estruturados de compilacao Typst.
