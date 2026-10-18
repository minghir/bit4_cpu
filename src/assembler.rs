use std::fs;

pub fn assemble(source_path: &str, output_path: &str) -> Result<(), String> {
    let content = fs::read_to_string(source_path)
        .map_err(|e| format!("Eroare la citirea fișierului sursă: {}", e))?;
    
    let mut machine_code = Vec::new();
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        let clean_line = line.trim();
        if clean_line.is_empty() || clean_line.starts_with(';') {
            continue;
        }

        let code_part = clean_line.split(';').next().unwrap().trim();
        if code_part.is_empty() {
            continue;
        }

        // Împărțim după spații SAU virgule pentru a curăța ușor argumentele (ex: "P0," devine "P0")
        let parts: Vec<String> = code_part
            .split(|c: char| c.is_whitespace() || c == ',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if parts.is_empty() {
            continue;
        }

        let mnemonic = parts[0].to_uppercase();

        // Helper pentru parsarea numerelor (zecimal sau hexazecimal)
        let parse_num = |s: &str| -> Result<u16, String> {
            if s.starts_with("0x") || s.starts_with("0X") {
                u16::from_str_radix(&s[2..], 16)
                    .map_err(|e| format!("Eroare parsare hexa '{}': {}", s, e))
            } else {
                s.parse::<u16>()
                    .map_err(|e| format!("Eroare parsare zecimală '{}': {}", s, e))
            }
        };

        match mnemonic.as_str() {
            "NOP" => {
                machine_code.push("00".to_string());
            }
            "INC" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: Instrucțiunea INC cere un registru", line_num));
                }
                let reg_str = parts[1].trim_start_matches('R').trim_start_matches('r');
                let reg = parse_num(reg_str)? as u8;
                let byte = 0x60 | (reg & 0x0F); // Corectat la 0x60 conform tabelului
                machine_code.push(format!("{:02X}", byte));
            }
            "BBL" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: Instrucțiunea BBL cere o valoare", line_num));
                }
                let val = parse_num(&parts[1])? as u8;
                let byte = 0xC0 | (val & 0x0F); // Corectat la 0xC0 conform tabelului
                machine_code.push(format!("{:02X}", byte));
            }
            "FIM" => {
                if parts.len() < 3 {
                    return Err(format!("Eroare linia {}: FIM cere o pereche și o valoare (ex: FIM P0, 0xAB)", line_num));
                }
                let pair_str = parts[1].trim_start_matches('P').trim_start_matches('p');
                let pair_idx = parse_num(pair_str)? as u8;
                let val8 = parse_num(&parts[2])? as u8;

                let byte1 = 0x20 | ((pair_idx & 0x07) << 1);
                machine_code.push(format!("{:02X}", byte1));
                machine_code.push(format!("{:02X}", val8));
            }
            "JUN" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: JUN cere o adresă", line_num));
                }
                let addr = parse_num(&parts[1])?;
                let byte1 = 0x40 | (((addr >> 8) & 0x0F) as u8);
                let byte2 = (addr & 0xFF) as u8;

                machine_code.push(format!("{:02X}", byte1));
                machine_code.push(format!("{:02X}", byte2));
            }
            "JMS" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: JMS cere o adresă", line_num));
                }
                let addr = parse_num(&parts[1])?;
                let byte1 = 0x30 | (((addr >> 8) & 0x0F) as u8);
                let byte2 = (addr & 0xFF) as u8;

                machine_code.push(format!("{:02X}", byte1));
                machine_code.push(format!("{:02X}", byte2));
            }
			"ADD" => {
                // ADD r -> 1 octet: Opcode 0x8 + r (0..15)
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: Instrucțiunea ADD cere un registru", line_num));
                }
                let reg_str = parts[1].trim_start_matches('R').trim_start_matches('r');
                let reg = parse_num(reg_str)? as u8;
                let byte = 0x80 | (reg & 0x0F);
                machine_code.push(format!("{:02X}", byte));
            }
            "SUB" => {
                // SUB r -> 1 octet: Opcode 0x9 + r (0..15)
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: Instrucțiunea SUB cere un registru", line_num));
                }
                let reg_str = parts[1].trim_start_matches('R').trim_start_matches('r');
                let reg = parse_num(reg_str)? as u8;
                let byte = 0x90 | (reg & 0x0F);
                machine_code.push(format!("{:02X}", byte));
            }
			"LDM" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: LDM cere o valoare", line_num));
                }
                let val = parse_num(&parts[1])? as u8;
                let byte = 0xD0 | (val & 0x0F);
                machine_code.push(format!("{:02X}", byte));
            }
            "WRM" => {
                machine_code.push("E0".to_string());
            }
            "RDM" => {
                machine_code.push("E3".to_string());
            }
			"JCN" => {
                // JCN cond, addr8 -> 2 octeți: Byte 1 = 0x10 | cond, Byte 2 = addr8
                if parts.len() < 3 {
                    return Err(format!("Eroare linia {}: JCN cere o condiție și o adresă (ex: JCN 0x4, 0x10)", line_num));
                }
                let cond = parse_num(&parts[1])? as u8;
                let addr8 = parse_num(&parts[2])? as u8;

                let byte1 = 0x10 | (cond & 0x0F);
                machine_code.push(format!("{:02X}", byte1));
                machine_code.push(format!("{:02X}", addr8));
            }
			"ISZ" => {
                // ISZ r, addr8 -> 2 octeți: Byte 1 = 0x70 | r, Byte 2 = addr8
                if parts.len() < 3 {
                    return Err(format!("Eroare linia {}: ISZ cere un registru și o adresă (ex: ISZ R0, 0x03)", line_num));
                }
                let reg_str = parts[1].trim_start_matches('R').trim_start_matches('r');
                let reg = parse_num(reg_str)? as u8;
                let addr8 = parse_num(&parts[2])? as u8;

                let byte1 = 0x70 | (reg & 0x0F);
                machine_code.push(format!("{:02X}", byte1));
                machine_code.push(format!("{:02X}", addr8));
            }
			"CLC" => {
                machine_code.push("F0".to_string());
            }
            "STC" => {
                machine_code.push("F1".to_string());
            }
            "IAC" => {
                machine_code.push("F2".to_string());
            }
            "DAC" => {
                machine_code.push("F3".to_string());
            }
			"RAL" => {
                machine_code.push("F4".to_string());
            }
            "RAR" => {
                machine_code.push("F5".to_string());
            }
			"LD" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: Instrucțiunea LD cere un registru", line_num));
                }
                let reg_str = parts[1].trim_start_matches('R').trim_start_matches('r');
                let reg = parse_num(reg_str)? as u8;
                let byte = 0xA0 | (reg & 0x0F);
                machine_code.push(format!("{:02X}", byte));
            }
            "XCH" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: Instrucțiunea XCH cere un registru", line_num));
                }
                let reg_str = parts[1].trim_start_matches('R').trim_start_matches('r');
                let reg = parse_num(reg_str)? as u8;
                let byte = 0xB0 | (reg & 0x0F);
                machine_code.push(format!("{:02X}", byte));
            }
			"SRC" => {
                if parts.len() < 2 {
                    return Err(format!("Eroare linia {}: SRC cere o pereche (ex: SRC P0)", line_num));
                }
                let pair_str = parts[1].trim_start_matches('P').trim_start_matches('p');
                let pair_idx = parse_num(pair_str)? as u8;
                
                // Opcode-ul pentru SRC este impar: 0x21 | (pair_idx << 1)
                let byte = 0x21 | ((pair_idx & 0x07) << 1);
                machine_code.push(format!("{:02X}", byte));
            }
			"ADM" => {
                machine_code.push("E5".to_string());
            }
            "SBM" => {
                machine_code.push("E6".to_string());
            }
			"WMP" => { machine_code.push("E1".to_string()); }
            "WRR" => { machine_code.push("E2".to_string()); }
            "RDR" => { machine_code.push("E4".to_string()); }
            "WDR" => { machine_code.push("E7".to_string()); }
			"HALT" => {
                // HALT -> 0xFF sau 0x00 (să zicem 0x00 pentru NOP/HALT, sau un cod dedicat)
                machine_code.push("FF".to_string()); // Folosim 0xFF pentru HALT clar
            }
			
            _ => {
                return Err(format!("Eroare pe linia {}: Mnemonică necunoscută '{}'", line_num, mnemonic));
            }
        }
    }

    fs::write(output_path, machine_code.join("\n") + "\n")
        .map_err(|e| format!("Eroare la scrierea fișierului de ieșire: {}", e))?;

    println!("[Asamblor] Sursă asamblată cu succes în '{}'!", output_path);
    Ok(())
}