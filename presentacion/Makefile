MAIN = presentacion
LATEXMK = latexmk
FLAGS = -lualatex -interaction=nonstopmode -halt-on-error -file-line-error

.PHONY: all clean purge watch

all: $(MAIN).pdf

$(MAIN).pdf: $(wildcard *.tex) $(wildcard figures/*.png)
	$(LATEXMK) $(FLAGS) $(MAIN).tex

watch:
	$(LATEXMK) $(FLAGS) -pvc $(MAIN).tex

clean:
	$(LATEXMK) -c $(MAIN).tex

purge: clean
	rm -f $(MAIN).pdf
