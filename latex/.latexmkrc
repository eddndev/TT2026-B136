# .latexmkrc — integra makeglossaries y biber en el flujo de latexmk
$pdf_mode = 4;            # 4 = lualatex
$lualatex = 'lualatex -interaction=nonstopmode -halt-on-error -file-line-error %O %S';
$bibtex_use = 2;         # usa biber/bibtex y limpia .bbl

# --- Dependencia de glossaries: ejecutar makeglossaries ---
add_cus_dep('glo', 'gls', 0, 'run_makeglossaries');
add_cus_dep('acn', 'acr', 0, 'run_makeglossaries');
sub run_makeglossaries {
    my ($base_name, $path) = fileparse($_[0]);
    my $cmd = "makeglossaries -d \"$path\" \"$base_name\"";
    return system($cmd);
}
push @generated_exts, 'glo', 'gls', 'glg', 'acn', 'acr', 'alg';
$clean_ext .= ' %R.ist %R.xdy';
