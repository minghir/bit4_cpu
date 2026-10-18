use std::fs;

mod assembler;

#[allow(dead_code)]
struct Intel4004 {
    ac: u8,             // Acumulatorul pe 4 biți (0x0..=0xF)
    cy: bool,           // Carry flag (pe 1 bit)
    regs: [u8; 16],     // 16 registre individuale pe 4 biți (R0..R15)
    pc: u16,            // Program Counter pe 12 biți (0x000..=0xFFF)
    stack: [u16; 3],    // Stivă hardware pe 3 niveluri pentru subrutine
    rom: [u8; 4096],    // Memorie ROM de 4 KB
    ram: [u8; 256],     // Memorie RAM simplificată pe 8 biți pentru date
	rom_ports: [u8; 16], // Porturile I/O ROM simulate (0..15)
    ram_ports: [u8; 16], // Porturile I/O RAM simulate (0..15)
	src_address: u8,    // Adresa RAM/IO selectată prin SRC
	test_pin: bool,     // Pinul extern de test pentru JCN
}

impl Intel4004 {
    fn new() -> Self {
        Self {
            ac: 0x0,
            cy: false,
            regs: [0x0; 16],
            pc: 0x0000,
            stack: [0x0000; 3],
            rom: [0x00; 4096],
            ram: [0x00; 256],
			rom_ports: [0x00; 16],
			ram_ports: [0x00; 16],
			src_address: 0x00,
			test_pin: false,
        }
    }

    fn get_pair(&self, pair_idx: usize) -> u8 {
        let hi = self.regs[pair_idx * 2] & 0x0F;
        let lo = self.regs[pair_idx * 2 + 1] & 0x0F;
        (hi << 4) | lo
    }

    fn set_pair(&mut self, pair_idx: usize, val: u8) {
        self.regs[pair_idx * 2] = (val >> 4) & 0x0F;
        self.regs[pair_idx * 2 + 1] = val & 0x0F;
    }

    fn push_stack(&mut self, addr: u16) {
        self.stack[2] = self.stack[1];
        self.stack[1] = self.stack[0];
        self.stack[0] = addr & 0x0FFF;
    }

    fn pop_stack(&mut self) -> u16 {
        let addr = self.stack[0];
        self.stack[0] = self.stack[1];
        self.stack[1] = self.stack[2];
        self.stack[2] = 0x0000;
        addr
    }

    fn step(&mut self) -> bool {
        let pc_val = self.pc as usize;
        if pc_val >= self.rom.len() {
            println!("[CPU] Eroare: PC a depășit limita ROM-ului (4KB)!");
            return false;
        }

        let byte1 = self.rom[pc_val];

        if byte1 == 0xFF {
            println!("[PC: 0x{:03X}] HALT întâlnit. Oprire.", self.pc);
            return false;
        }

        let high_nibble = (byte1 >> 4) & 0x0F;

        match high_nibble {
            0x0 => {
                println!("[PC: 0x{:03X}] NOP", self.pc);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
            0x2 => {
                // Familia 0x20..0x2F: FIM (pare) sau SRC (impare)
                let is_src = (byte1 & 0x01) != 0;
                let pair_idx = (byte1 & 0x0E) >> 1;

                if is_src {
                    // SRC pair (Send Register Control)
                    let addr = self.get_pair(pair_idx as usize);
                    self.src_address = addr;
                    println!("[PC: 0x{:03X}] SRC P{} -> Adresă selectată 0x{:02X}", self.pc, pair_idx, addr);
                    self.pc = (self.pc + 1) & 0x0FFF;
                } else {
                    // FIM pair, data (2 octeți)
                    let byte2 = self.rom[(self.pc + 1) as usize];
                    self.set_pair(pair_idx as usize, byte2);
                    println!("[PC: 0x{:03X}] FIM P{} -> valoare 0x{:02X}", self.pc, pair_idx, byte2);
                    self.pc = (self.pc + 2) & 0x0FFF;
                }
            }
            0x3 => {
                let byte2 = self.rom[(self.pc + 1) as usize] as u16;
                let target_addr = (((byte1 & 0x0F) as u16) << 8) | byte2;
                let return_addr = (self.pc + 2) & 0x0FFF;
                self.push_stack(return_addr);
                
                println!("[PC: 0x{:03X}] JMS la subrutina 0x{:03X} (Retur: 0x{:03X})", self.pc, target_addr, return_addr);
                self.pc = target_addr & 0x0FFF;
            }
            0x4 => {
                let byte2 = self.rom[(self.pc + 1) as usize] as u16;
                let target_addr = (((byte1 & 0x0F) as u16) << 8) | byte2;
                
                println!("[PC: 0x{:03X}] JUN necondiționat la 0x{:03X}", self.pc, target_addr);
                self.pc = target_addr & 0x0FFF;
            }
            0x8 => {
                let reg_idx = (byte1 & 0x0F) as usize;
                let reg_val = self.regs[reg_idx] as u16;
                let ac_val = self.ac as u16;
                let carry_val = if self.cy { 1 } else { 0 };

                let sum = ac_val + reg_val + carry_val;
                self.cy = sum > 0x0F;
                self.ac = (sum & 0x0F) as u8;

                println!("[PC: 0x{:03X}] ADD R{} -> AC devine 0x{:X} | Carry: {}", self.pc, reg_idx, self.ac, self.cy);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
            0x9 => {
                let reg_idx = (byte1 & 0x0F) as usize;
                let reg_val = self.regs[reg_idx] as i32;
                let ac_val = self.ac as i32;
                let borrow_val = if self.cy { 1 } else { 0 };

                let diff = ac_val - reg_val - borrow_val;
                self.cy = diff < 0;
                self.ac = ((diff + 16) % 16) as u8;

                println!("[PC: 0x{:03X}] SUB R{} -> AC devine 0x{:X} | Borrow/CY: {}", self.pc, reg_idx, self.ac, self.cy);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
            0xC => {
                let imm_val = byte1 & 0x0F;
                self.ac = imm_val;
                let return_addr = self.pop_stack();
                
                println!("[PC: 0x{:03X}] BBL -> AC=0x{:X}, retur la 0x{:03X}", self.pc, self.ac, return_addr);
                self.pc = return_addr;
            }
            0xD => {
                self.ac = byte1 & 0x0F;
                println!("[PC: 0x{:03X}] LDM 0x{:X} -> AC devine 0x{:X}", self.pc, self.ac, self.ac);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
            0xE => {
                let sub_op = byte1 & 0x0F;
                match sub_op {
                    0x0 => {
                        // WRM: Scrie AC în RAM la adresa selectată prin SRC
                        let addr = self.src_address as usize;
                        if addr < self.ram.len() {
                            self.ram[addr] = self.ac & 0x0F;
                            println!("[PC: 0x{:03X}] WRM -> Scris AC (0x{:X}) în RAM[0x{:02X}]", self.pc, self.ac, addr);
                        }
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x3 => {
                        // RDM: Citește din RAM[SRC] în AC
                        let addr = self.src_address as usize;
                        if addr < self.ram.len() {
                            self.ac = self.ram[addr] & 0x0F;
                            println!("[PC: 0x{:03X}] RDM -> Citit din RAM[0x{:02X}] în AC (0x{:X})", self.pc, addr, self.ac);
                        }
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x5 => {
                        // ADM: Adună conținutul RAM[SRC] la AC cu Carry
                        let addr = self.src_address as usize;
                        if addr < self.ram.len() {
                            let ram_val = self.ram[addr] as u16;
                            let ac_val = self.ac as u16;
                            let carry_val = if self.cy { 1 } else { 0 };

                            let sum = ac_val + ram_val + carry_val;
                            self.cy = sum > 0x0F;
                            self.ac = (sum & 0x0F) as u8;

                            println!("[PC: 0x{:03X}] ADM -> Adăugat RAM[0x{:02X}] (0x{:X}) la AC | Rezultat: 0x{:X} | Carry: {}", 
                                self.pc, addr, self.ram[addr], self.ac, self.cy);
                        }
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x6 => {
                        // SBM: Scade conținutul RAM[SRC] din AC cu Borrow
                        let addr = self.src_address as usize;
                        if addr < self.ram.len() {
                            let ram_val = self.ram[addr] as i32;
                            let ac_val = self.ac as i32;
                            let borrow_val = if self.cy { 1 } else { 0 };

                            let diff = ac_val - ram_val - borrow_val;
                            self.cy = diff < 0;
                            self.ac = ((diff + 16) % 16) as u8;

                            println!("[PC: 0x{:03X}] SBM -> Scăzut RAM[0x{:02X}] (0x{:X}) din AC | Rezultat: 0x{:X} | Borrow/CY: {}", 
                                self.pc, addr, self.ram[addr], self.ac, self.cy);
                        }
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x1 => {
                        // WMP: Scrie AC în portul de memorie RAM selectat prin SRC
                        let port_idx = (self.src_address & 0x0F) as usize;
                        self.ram_ports[port_idx] = self.ac & 0x0F;
                        println!("[PC: 0x{:03X}] WMP -> Scris AC (0x{:X}) în portul RAM [{}]", self.pc, self.ac, port_idx);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x2 => {
                        // WRR: Scrie AC în portul de ieșire RAM/ROM selectat prin SRC
                        let port_idx = (self.src_address & 0x0F) as usize;
                        self.ram_ports[port_idx] = self.ac & 0x0F;
                        println!("[PC: 0x{:03X}] WRR -> Scris AC (0x{:X}) în portul WRR [{}]", self.pc, self.ac, port_idx);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x4 => {
                        // RDR: Citește portul de I/O ROM selectat prin SRC în AC
                        let port_idx = (self.src_address & 0x0F) as usize;
                        self.ac = self.rom_ports[port_idx] & 0x0F;
                        println!("[PC: 0x{:03X}] RDR -> Citit portul ROM [{}] în AC (0x{:X})", self.pc, port_idx, self.ac);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x7 => {
                        // WDR: Scrie AC în portul de I/O ROM selectat prin SRC
                        let port_idx = (self.src_address & 0x0F) as usize;
                        self.rom_ports[port_idx] = self.ac & 0x0F;
                        println!("[PC: 0x{:03X}] WDR -> Scris AC (0x{:X}) în portul ROM [{}]", self.pc, self.ac, port_idx);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    _ => {
                        println!("[PC: 0x{:03X}] Opcode 0xE necunoscut: 0x{:02X}", self.pc, byte1);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                }
            }
			0x1 => {
                // JCN cond, addr8 (Jump Conditional - 2 octeți)
                let cond_bits = byte1 & 0x0F;
                let target_addr8 = self.rom[(self.pc + 1) as usize];

                let invert = (cond_bits & 0x8) != 0;
                let test_z = (cond_bits & 0x4) != 0;
                let test_cy = (cond_bits & 0x2) != 0;
                let test_t = (cond_bits & 0x1) != 0;

                let mut condition_met = false;
                if test_z && (self.ac == 0) { condition_met = true; }
                if test_cy && self.cy { condition_met = true; }
                if test_t && self.test_pin { condition_met = true; }

                if invert {
                    condition_met = !condition_met;
                }

                if condition_met {
                    // Salt în cadrul aceleiași pagini de 256 de octeți (păstrând nibble-ul superior al PC-ului)
                    let current_page = self.pc & 0xF00;
                    let target_addr = current_page | (target_addr8 as u16);
                    println!("[PC: 0x{:03X}] JCN îndeplinit -> Salt la 0x{:03X}", self.pc, target_addr);
                    self.pc = target_addr & 0x0FFF;
                } else {
                    println!("[PC: 0x{:03X}] JCN neîndeplinit -> Continuă", self.pc);
                    self.pc = (self.pc + 2) & 0x0FFF;
                }
            }
			0x6 => {
                // INC r: Incrementează direct un registru (1 octet: 0x60 | r)
                let reg_idx = (byte1 & 0x0F) as usize;
                self.regs[reg_idx] = (self.regs[reg_idx] + 1) & 0x0F;
                println!("[PC: 0x{:03X}] INC R{} -> devine 0x{:X}", self.pc, reg_idx, self.regs[reg_idx]);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
			0x7 => {
                // ISZ r, addr8 (Increment and Skip if Zero - 2 octeți)
                let reg_idx = (byte1 & 0x0F) as usize;
                let target_addr8 = self.rom[(self.pc + 1) as usize];

                // Incrementează registrul modulo 16
                self.regs[reg_idx] = (self.regs[reg_idx] + 1) & 0x0F;
                let result = self.regs[reg_idx];

                if result != 0 {
                    let current_page = self.pc & 0xF00;
                    let target_addr = current_page | (target_addr8 as u16);
                    println!("[PC: 0x{:03X}] ISZ R{} -> devine 0x{:X} (Ne-zero) -> Salt la 0x{:03X}", self.pc, reg_idx, result, target_addr);
                    self.pc = target_addr & 0x0FFF;
                } else {
                    println!("[PC: 0x{:03X}] ISZ R{} -> devine 0x0 (Zero) -> Continuă", self.pc, reg_idx);
                    self.pc = (self.pc + 2) & 0x0FFF;
                }
            }
			0xF => {
                // Instrucțiuni din familia 0xF (CLC, STC, IAC, DAC, etc.)
                let sub_op = byte1 & 0x0F;
                match sub_op {
                    0x0 => {
                        // CLC: Clear Carry
                        self.cy = false;
                        println!("[PC: 0x{:03X}] CLC -> Carry setat la false", self.pc);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x1 => {
                        // STC: Set Carry
                        self.cy = true;
                        println!("[PC: 0x{:03X}] STC -> Carry setat la true", self.pc);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x2 => {
                        // IAC: Increment Accumulator
                        let sum = (self.ac as u16) + 1;
                        self.cy = sum > 0x0F;
                        self.ac = (sum & 0x0F) as u8;
                        println!("[PC: 0x{:03X}] IAC -> AC devine 0x{:X} | Carry: {}", self.pc, self.ac, self.cy);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                    0x3 => {
                        // DAC: Decrement Accumulator
                        let diff = (self.ac as i32) - 1;
                        self.cy = diff >= 0;
                        self.ac = ((diff + 16) % 16) as u8;
                        println!("[PC: 0x{:03X}] DAC -> AC devine 0x{:X}", self.pc, self.ac);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
					0x4 => {
                        // RAL: Rotate Accumulator Left through Carry
                        // Bitul cel mai din stânga (bitul 3) intră în Carry, vechiul Carry intră în bitul 0 al AC
                        let ac_val = self.ac & 0x0F;
                        let old_cy = if self.cy { 1 } else { 0 };
                        let new_cy = (ac_val & 0x08) != 0; // Bitul 3 devine noul Carry
                        
                        let new_ac = ((ac_val << 1) | old_cy) & 0x0F;
                        self.cy = new_cy;
                        self.ac = new_ac;

                        println!("[PC: 0x{:03X}] RAL -> AC devine 0x{:X} | Carry: {}", self.pc, self.ac, self.cy);
                        self.pc = (self.pc + 1) & 0x0FFF;
					}
					0x5 => {
								// RAR: Rotate Accumulator Right through Carry
								// Bitul cel mai din dreapta (bitul 0) intră în Carry, vechiul Carry intră în bitul 3 al AC
								let ac_val = self.ac & 0x0F;
								let old_cy = if self.cy { 1 } else { 0 };
								let new_cy = (ac_val & 0x01) != 0; // Bitul 0 devine noul Carry
								
								let new_ac = ((ac_val >> 1) | (old_cy << 3)) & 0x0F;
								self.cy = new_cy;
								self.ac = new_ac;

								println!("[PC: 0x{:03X}] RAR -> AC devine 0x{:X} | Carry: {}", self.pc, self.ac, self.cy);
								self.pc = (self.pc + 1) & 0x0FFF;
					}
                    _ => {
                        println!("[PC: 0x{:03X}] Opcode 0xF necunoscut: 0x{:02X}", self.pc, byte1);
                        self.pc = (self.pc + 1) & 0x0FFF;
                    }
                }
            }
			0xA => {
                // LD r: AC = Registru
                let reg_idx = (byte1 & 0x0F) as usize;
                self.ac = self.regs[reg_idx] & 0x0F;
                println!("[PC: 0x{:03X}] LD R{} -> AC devine 0x{:X}", self.pc, reg_idx, self.ac);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
            0xB => {
                // XCH r: Schimbă valorile dintre AC și Registru
                let reg_idx = (byte1 & 0x0F) as usize;
                let temp = self.ac;
                self.ac = self.regs[reg_idx] & 0x0F;
                self.regs[reg_idx] = temp & 0x0F;
                println!("[PC: 0x{:03X}] XCH R{} -> AC devine 0x{:X}, R{} devine 0x{:X}", self.pc, reg_idx, self.ac, reg_idx, self.regs[reg_idx]);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
            _ => {
                println!("[PC: 0x{:03X}] Opcode necunoscut: 0x{:02X}", self.pc, byte1);
                self.pc = (self.pc + 1) & 0x0FFF;
            }
        }
        true
    }

    fn run(&mut self) {
        loop {
            if !self.step() {
                break;
            }
        }
    }
}

fn main() {
    println!("==========================================");
    println!("   ASAMBLOR & EMULATOR INTEL 4004         ");
    println!("==========================================");

    if let Err(e) = assembler::assemble("source.asm", "program.hex") {
        println!("Eroare de asamblare: {}", e);
        return;
    }

    let file_contents = match fs::read_to_string("program.hex") {
        Ok(content) => content,
        Err(e) => {
            println!("Eroare la citirea program.hex: {}", e);
            return;
        }
    };

    let mut cpu = Intel4004::new();
    let mut program = Vec::new();
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

    for (i, &byte) in program.iter().enumerate() {
        if i < cpu.rom.len() {
            cpu.rom[i] = byte;
        }
    }

    println!("\n[Sistem] Pornire execuție program asamblat...\n");
    cpu.run();
    println!("\n--- Oprire Execuție ---");
    println!("Valoare finală R0 = 0x{:X}", cpu.regs[0]);
    println!("Valoare finală R1 = 0x{:X}", cpu.regs[1]);
    println!("Perechea P0 (R0-R1) = 0x{:02X}", cpu.get_pair(0));
    println!("Valoare finală AC = 0x{:X}", cpu.ac);
}