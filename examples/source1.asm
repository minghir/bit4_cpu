# ==========================================
# Program scris în limbaj de asamblare propriu
# ==========================================

LDI 5     # Încarcă 5 în Acumulator
ST 0      # Salvează în R0
LD 0      # Încarcă din R0 înapoi
INC 0     # Incrementează R0 (devine 6)
LD 0      # Încarcă noua valoare în AC
STORE 2   # Salvează în RAM[2]
OUT 2     # Afișează RAM[2]
HALT      # Oprește execuția