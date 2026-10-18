; ==========================================
; Test RAM 4002 (Partea 1 - Adaptat)
; ==========================================

FIM P0, 0       ; Inițializează P0 (R0=0, R1=0)
FIM P1, 0       ; Inițializează P1 (R2=0, R3=0)
LDM 12          ; Încarcă 12 în Acumulator (0xC)
XCH R2          ; R2 devine 12 (12 pași / rânduri)

; -- Aici începe bucla (adresa 0x006) --
SRC P0          ; Selectează registrul și adresa RAM
WRM             ; Scrie AC în memoria RAM
IAC             ; Incrementează AC
ISZ R1, 0x006   ; Dacă R1 nu e zero, sare înapoi la 0x006 (loop1)
INC R0          ; Incrementează R0
ISZ R2, 0x006   ; Dacă R2 nu e zero, sare înapoi la 0x006
HALT            ; Oprire