// keyboard set, key[0] is without shift, key[1] if when shifted
// more than one indexes with '\0' as value because are reserved
pub(super) static KEYMAP: [[char; 2]; 58] = [
    ['\0', '\0'],
    ['\x1B', '\x1B'],
    ['1', '!'],
    ['2', '@'],
    ['3', '#'],
    ['4', '$'],
    ['5', '%'],
    ['6', '^'],
    ['7', '&'],
    ['8', '*'],
    ['9', '('],
    ['0', ')'],
    ['-', '_'],
    ['=', '+'],
    ['\x7F', '\x7F'],
    ['\t', '\t'],
    ['q', 'Q'],
    ['w', 'W'],
    ['e', 'E'],
    ['r', 'R'],
    ['t', 'T'],
    ['y', 'Y'],
    ['u', 'U'],
    ['i', 'I'],
    ['o', 'O'],
    ['p', 'P'],
    ['[', '{'],
    [']', '}'],
    ['\n', '\n'],
    ['\0', '\0'],
    ['a', 'A'],
    ['s', 'S'],
    ['d', 'D'],
    ['f', 'F'],
    ['g', 'G'],
    ['h', 'H'],
    ['j', 'J'],
    ['k', 'K'],
    ['l', 'L'],
    [';', ':'],
    ['\'', '"'],
    ['`', '~'],
    ['\0', '\0'],
    ['\\', '|'],
    ['z', 'Z'],
    ['x', 'X'],
    ['c', 'C'],
    ['v', 'V'],
    ['b', 'B'],
    ['n', 'N'],
    ['m', 'M'],
    [',', '<'],
    ['.', '>'],
    ['/', '?'],
    ['\0', '\0'],
    ['\0', '\0'],
    ['\0', '\0'],
    [' ', ' '],
];

/// function that returns the key index in US array
///
/// # Arguments
/// - `scancode` - the scancode of the char
///
/// # Returns
/// - the index of the char or 0 (the index of '\0') otherwise
pub fn get_key_index(scancode: u8) -> usize {
    match scancode {
        b'\x1B' => 1,
        b'1' => 2,
        b'2' => 3,
        b'3' => 4,
        b'4' => 5,
        b'5' => 6,
        b'6' => 7,
        b'7' => 8,
        b'8' => 9,
        b'9' => 10,
        b'0' => 11,
        b'-' => 12,
        b'=' => 13,
        b'\x7F' => 14,
        b'\t' => 15,
        b'q' => 16,
        b'w' => 17,
        b'e' => 18,
        b'r' => 19,
        b't' => 20,
        b'y' => 21,
        b'u' => 22,
        b'i' => 23,
        b'o' => 24,
        b'p' => 25,
        b'[' => 26,
        b']' => 27,
        b'\n' => 28,
        b'a' => 30,
        b's' => 31,
        b'd' => 32,
        b'f' => 33,
        b'g' => 34,
        b'h' => 35,
        b'j' => 36,
        b'k' => 37,
        b'l' => 38,
        b';' => 39,
        b'\'' => 40,
        b'`' => 41,
        b'\\' => 43,
        b'z' => 44,
        b'x' => 45,
        b'c' => 46,
        b'v' => 47,
        b'b' => 48,
        b'n' => 49,
        b'm' => 50,
        b',' => 51,
        b'.' => 52,
        b'/' => 53,
        b' ' => 57,
        _ => 0,
    }
}
