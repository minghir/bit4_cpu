use std::fs;

// Spunem compilatorului Rust să includă și să folosească codul din fișierul 'assembler.rs'
mod assembler;

// Structura care definește starea internă a procesorului nostru virtual
struct Cpu4Bit {
    ac: u8,             // Acumulatorul pe 4 biți (valori 0..15)
    regs: [u8; 16],     // Banc de 16 registre interne (R0..R15)
    pc: u8,             // Program Counter (ține minte la ce adresă din ROM suntem)
    ram: [u8; 16],      // Memoria RAM (16 celule)
    rom: [u8; 32],      // Memoria ROM (32 octeți pentru instrucțiuni)
    zero_flag: bool,    // Flag de Zero (devine true dacă AC == 0)
    carry_flag: bool,   // Flag de Transport/Borrow (pentru depășiri aritmetice)
}

// 'impl' (Implementation) leagă funcțiile direct de structura noastră Cpu4Bit (cum sunt metodele în OOP)
impl Cpu4Bit {
    // Funcție de inițializare a unui nou procesor (constructor)
    fn new() -> Self {
        Self {
            ac: 0x0,
            regs: [0x0; 16],
            pc: 0x0,
            ram: [0x0; 16],
            rom: [0x0; 32],
            zero_flag: false,
            carry_flag: false,
        }
    }

    // Execută un singur ciclu al procesorului: Fetch -> Decode -> Execute
    // '&mut self' înseamnă că această funcție are voie să modifice starea internă a procesorului
    fn step(&mut self) -> bool {
        // Verificăm să nu ieșim din memoria ROM
        if (self.pc as usize) >= self.rom.len() {
            println!("[CPU] Eroare: PC a depășit dimensiunea ROM-ului!");
            return false;
        }

        // 1. FETCH: Luăm instrucțiunea de la adresa indicată de Program Counter (PC)
        let instruction = self.rom[self.pc as usize];
        
        // 2. DECODE: Despărțim octetul în două părți de câte 4 biți (nibbles)
        let opcode = (instruction >> 4) & 0x0F; // Extragem primii 4 biți (instrucțiunea)
        let operand = instruction & 0x0F;       // Extragem ultimii 4 biți (valoarea sau adresa)

        // 3. EXECUTE: Executăm instrucțiunea în funcție de opcode
        match opcode {
            0x0 => {
                println!("[PC: 0x{:X}] HALT întâlnit. Oprire.", self.pc);
                return false; // Oprim execuția buclei
            }
            0x1 => {
                // LOAD: Copiază din RAM în Acumulator
                self.ac = self.ram[operand as usize] & 0x0F;
                self.update_flags();
                println!("[PC: 0x{:X}] LOAD RAM[0x{:X}] -> AC devine 0x{:X}", self.pc, operand, self.ac);
                self.pc += 1; // Avansăm PC-ul la următoarea instrucțiune
            }
            0x2 => {
                // STORE: Salvează Acumulatorul în RAM
                self.ram[operand as usize] = self.ac & 0x0F;
                println!("[PC: 0x{:X}] STORE AC (0x{:X}) în RAM[0x{:X}]", self.pc, self.ac, operand);
                self.pc += 1;
            }
            0x3 => {
                // ADD: Adună valoarea din RAM la Acumulator
                let sum = (self.ac as u16) + (self.ram[operand as usize] as u16);
                self.carry_flag = sum > 0x0F; // Dacă depășește 15, activăm Carry
                self.ac = (sum & 0x0F) as u8;  // Pstrăm doar primii 4 biți
                self.update_flags();
                println!("[PC: 0x{:X}] ADD RAM[0x{:X}] -> AC devine 0x{:X} | Carry: {}", self.pc, operand, self.ac, self.carry_flag);
                self.pc += 1;
            }
            0x4 => {
                // OUT: Afișează valoarea dintr-o adresă RAM
                println!("[PC: 0x{:X}] OUT -> Valoarea din RAM[0x{:X}] este 0x{:X}", self.pc, operand, self.ram[operand as usize]);
                self.pc += 1;
            }
            0x5 => {
                // SUB: Scade valoarea din RAM din Acumulator
                let ac_val = self.ac as i32;
                let mem_val = self.ram[operand as usize] as i32;
                let diff = ac_val - mem_val;
                self.carry_flag = diff < 0;
                self.ac = ((diff + 16) % 16) as u8; // Asigurăm bucla pe 4 biți
                self.update_flags();
                println!("[PC: 0x{:X}] SUB RAM[0x{:X}] -> AC devine 0x{:X} | Borrow: {}", self.pc, operand, self.ac, self.carry_flag);
                self.pc += 1;
            }
            0x6 => {
                // LDI: Încarcă ovaloare directă în Acumulator
                self.ac = operand & 0x0F;
                self.update_flags();
                println!("[PC: 0x{:X}] LDI -> AC setat direct la 0x{:X}", self.pc, self.ac);
                self.pc += 1;
            }
            0x7 => {
                // JZ (Jump if Zero): Sare la adresă dacă zero_flag este adevărat
                if self.zero_flag {
                    println!("[PC: 0x{:X}] JZ luat -> AC este 0, salt la adresa 0x{:X}", self.pc, operand);
                    self.pc = operand; // Schimbăm direct Program Counter-ul la noua adresă
                } else {
                    println!("[PC: 0x{:X}] JZ ignorat -> AC nu este 0", self.pc);
                    self.pc += 1;
                }
            }
            0x8 => {
                // JMP: Salt necondiționat
                println!("[PC: 0x{:X}] JMP necondiționat la adresa 0x{:X}", self.pc, operand);
                self.pc = operand;
            }
            0x9 => {
                // LD: Încarcă dintr-un registru intern în Acumulator
                let reg_idx = operand as usize;
                self.ac = self.regs[reg_idx] & 0x0F;
                self.update_flags();
                println!("[PC: 0x{:X}] LD R{} -> AC devine 0x{:X}", self.pc, reg_idx, self.ac);
                self.pc += 1;
            }
            0xA => {
                // ST: Salvează Acumulatorul într-un registru intern
                let reg_idx = operand as usize;
                self.regs[reg_idx] = self.ac & 0x0F;
                println!("[PC: 0x{:X}] ST R{} -> R{} primește valoarea 0x{:X}", self.pc, reg_idx, reg_idx, self.regs[reg_idx]);
                self.pc += 1;
            }
            0xB => {
                // INC: Incrementează un registru intern
                let reg_idx = operand as usize;
                self.regs[reg_idx] = (self.regs[reg_idx] + 1) & 0x0F;
                println!("[PC: 0x{:X}] INC R{} -> R{} devine 0x{:X}", self.pc, reg_idx, reg_idx, self.regs[reg_idx]);
                self.pc += 1;
            }
            _ => {
                println!("[PC: 0x{:X}] Opcode necunoscut: 0x{:X}", self.pc, opcode);
                self.pc += 1;
            }
        }
        true // Continuăm execuția
    }

    // Funcție ajutătoare care actualizează flag-ul de zero
    fn update_flags(&mut self) {
        self.zero_flag = self.ac == 0x0;
    }

    // Bucla infinită care rulează pașii procesorului până când întâlnește HALT
    fn run(&mut self) {
        loop {
            if !self.step() {
                break;
            }
        }
    }
}

// Punctul de intrare în program (funcția main)
fn main() {
    println!("========================================");
    println!("   ASAMBLOR & EMULATOR 4-BIT (RUST)     ");
    println!("========================================");

    // 1. Apelăm asamblorul extern să transforme 'source.asm' în 'program.txt'
    // 'if let Err' verifică dacă a apărut o eroare, caz în care o afișează și oprește programul
    if let Err(e) = assembler::assemble("source.asm", "program.txt") {
        println!("Eroare de asamblare: {}", e);
        return;
    }

    // 2. Citim fișierul text program.txt generat de asamblor
    let file_contents = match fs::read_to_string("program.txt") {
        Ok(content) => content,
        Err(e) => {
            println!("Eroare la citirea program.txt: {}", e);
            return;
        }
    };

    // 3. Inițializăm procesorul
    let mut cpu = Cpu4Bit::new();
    let mut program = Vec::new();

    // Parsăm liniile din program.txt în octeți numerici
    for line in file_contents.lines() {
        let clean_line = line.trim();
        if clean_line.is_empty() {
            continue;
        }
        match u8::from_str_radix(clean_line, 16) {
            Ok(byte) => program.push(byte),
            Err(e) => println!("Eroare parsare hexa '{}': {}", clean_line, e),
        }
    }

    // Încărcăm programul în memoria ROM a procesorului
    for (i, &byte) in program.iter().enumerate() {
        if i < cpu.rom.len() {
            cpu.rom[i] = byte;
        }
    }

    println!("\n[Sistem] Pornire execuție program asamblat...\n");
    
    // Pornim simulatorul
    cpu.run();
    
    println!("\n--- Oprire Execuție ---");
    println!("Valoare finală în R0 = 0x{:X}", cpu.regs[0]);
    println!("Valoare finală în RAM[0x2] = 0x{:X}", cpu.ram[0x2]);
}