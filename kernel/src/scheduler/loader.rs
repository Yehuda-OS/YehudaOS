type Elf_Addr = u64; // Unsigned program address
type Elf_Off = u64; // Unsigned file offset
type Elf_Section = u16; // Unsigned section index
type Elf_Versym = u16; // Unsigned version symbol information
type Elf_Byte = u8;
type Elf_Half = u16;
type Elf_Sword = i32;
type Elf_Word = u32;
type Elf_Sxword = i64;
type Elf_Xword = u64;

const EI_NIDENT: usize = 16;
const EI_MAG0: u8 = 0x7f;
const EI_MAG1: u8 = 'E' as u8;
const EI_MAG2: u8 = 'L' as u8;
const EI_MAG3: u8 = 'F' as u8;

#[repr(C)]
struct Elf_Ehdr {
    e_idnt: [u8; EI_NIDENT],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: Elf_Addr,
    e_phoff: Elf_Off,
    e_shoff: Elf_Off,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}
