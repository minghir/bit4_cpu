use std::fs; // Importăm modulul de sistem de fișiere pentru a citi și scrie fișiere

// 'pub' înseamnă că această funcție este publică și poate fi apelată din main.rs
// 'Result<(), String>' este modul prin care Rust gestionează erorile: 
// fie se termină cu succes (()), fie returnează o eroare sub formă de text (String).
pub fn assemble(source_path: &str, output_path: &str) -> Result<(), String> {
    
    // Citim fișierul sursă text. 
    // '.map_err' convertește eroarea nativă a sistemului într-un String prietenos dacă ceva e în neregulă.
    let content = fs::read_to_string(source_path)
        .map_err(|e| format!("Eroare la citirea fișierului sursă: {}", e))?;
    
    // Creăm un vector dinamic (o listă care își poate schimba dimensiunea) pentru codul mașină
    let mut machine_code = Vec::new();

    // Iterăm prin fiecare linie din fișier, păstrând și numărul liniei (pentru erori)
    for (line_num, line) in content.lines().enumerate() {
        let clean_line = line.trim(); // Ștergem spațiile goale de la început și sfârșit
        
        // Ignorăm liniile goale sau cele care încep cu comentariul '#'
        if clean_line.is_empty() || clean_line.starts_with('#') {
            continue;
        }

        // Dacă există un comentariu pe aceeași linie, luăm doar partea de dinainte de '#'
        let code_part = clean_line.split('#').next().unwrap().trim();
        if code_part.is_empty() {
            continue;
        }

        // Împărțim linia după spații (ex: "LDI 5" devine ["LDI", "5"])
        let parts: Vec<&str> = code_part.split_whitespace().collect();
        let mnemonic = parts[0].to_uppercase(); // Facem litere mari pentru siguranță
        
        // Dacă există un al doilea element, acela este operandul (valoarea sau adresa)
        let operand: u8 = if parts.len() > 1 {
            let op_str = parts[1];
            // Verificăm dacă este scris în hexazecimal (cu prefixul 0x) sau zecimal normal
            if op_str.starts_with("0x") || op_str.starts_with("0X") {
                u8::from_str_radix(&op_str[2..], 16).unwrap_or(0)
            } else {
                op_str.parse().unwrap_or(0)
            }
        } else {
            0 // Dacă nu are operand, default este 0
        };

        // 'match' în Rust este ca un 'switch' mult mai puternic și sigur
        // Traducem mnemonica text în codul ei numeric (opcode pe 4 biți)
        let opcode = match mnemonic.as_str() {
            "HALT"  => 0x0, // Oprește execuția procesorului
            "LOAD"  => 0x1, // Încarcă o valoare din RAM în Acumulator (AC)
            "STORE" => 0x2, // Salvează Acumulatorul (AC) în RAM
            "ADD"   => 0x3, // Adună valoarea din RAM la Acumulator
            "OUT"   => 0x4, // Afișează sau trimite la ieșire valoarea din RAM
            "SUB"   => 0x5, // Scade valoarea din RAM din Acumulator
            "LDI"   => 0x6, // Încarcă o valoare directă (immediate) în Acumulator
            "JZ"    => 0x7, // Salt condiționat: sare dacă flag-ul Zero este activ (AC == 0)
            "JMP"   => 0x8, // Salt necondiționat la o adresă din ROM
            "LD"    => 0x9, // Încarcă valoarea dintr-un registru intern (ex: R0) în Acumulator
            "ST"    => 0xA, // Salvează Acumulatorul într-un registru intern (ex: R0)
            "INC"   => 0xB, // Incrementează cu 1 valoarea dintr-un registru intern
			"CALL"  => 0xC, // Apel de subrutină (salvează PC pe stivă și sare)
            "RET"   => 0xD, // Întoarcere din subrutină (scoate adresa de pe stivă)
            _ => {
                // Cazul implicit ('_'): Dacă întâlnim un cuvânt necunoscut, 
                // oprim execuția și returnăm o eroare care include numărul liniei
                return Err(format!("Eroare pe linia {}: Mnemonică necunoscută '{}'", line_num + 1, mnemonic));
            }
        };

        // Combinăm opcode-ul (4 biți superiori) cu operandul (4 biți inferiori) într-un singur octet (8 biți)
        // Operatorul '<< 4' mută opcode-ul la stânga cu 4 poziții, iar '|' face un OR binar
        let byte = ((opcode & 0x0F) << 4) | (operand & 0x0F);
        
        // Salvăm octetul format sub formă de text hexazecimal (ex: "65") în listă
        machine_code.push(format!("{:02X}", byte));
    }

    // Scriem toate instrucțiunile transformate în fișierul de ieșire (program.hex)
    fs::write(output_path, machine_code.join("\n") + "\n")
        .map_err(|e| format!("Eroare la scrierea fișierului de ieșire: {}", e))?;

    println!("[Asamblor] Fișierul '{}' a fost asamblat cu succes în '{}'!", source_path, output_path);
    Ok(()) // Returnăm succes
}