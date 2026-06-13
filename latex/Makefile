# Makefile — Trabajo Terminal 2026-B136
# Uso:  make        -> compila main.pdf con LuaLaTeX (latexmk)
#       make clean  -> borra auxiliares
#       make purge  -> borra auxiliares y el PDF

MAIN = main
LATEXMK = latexmk
FLAGS = -lualatex -interaction=nonstopmode -halt-on-error -file-line-error

.PHONY: all clean purge watch

all: $(MAIN).pdf

$(MAIN).pdf: $(MAIN).tex $(wildcard chapters/*.tex) $(wildcard config/*.tex) $(wildcard frontmatter/*.tex) references.bib
	$(LATEXMK) $(FLAGS) $(MAIN).tex

watch:
	$(LATEXMK) $(FLAGS) -pvc $(MAIN).tex

clean:
	$(LATEXMK) -c $(MAIN).tex
	rm -f $(MAIN).bbl $(MAIN).run.xml $(MAIN).glo $(MAIN).gls $(MAIN).glg \
	      $(MAIN).acn $(MAIN).acr $(MAIN).alg $(MAIN).ist

purge: clean
	rm -f $(MAIN).pdf
