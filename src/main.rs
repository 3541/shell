extern crate nix;

use std::io;
use std::process::Command;
use std::io::Write;
use std::io::Read;
use nix::sys::termios;

const LINE_BUF_LEN: usize = 50;
const RECALL_BUF_LEN: usize = 50;


mod builtin;

fn main() {
    let term_save = termios::tcgetattr(0).unwrap();
    let mut term = term_save;
    term.c_lflag.remove(termios::ICANON);
    term.c_lflag.remove(termios::ISIG);
    term.c_lflag.remove(termios::ECHO);
    termios::tcsetattr(0, termios::TCSADRAIN, &term).unwrap();
    exec_loop(&term_save);

/*    termios::tcsetattr(0, termios::TCSADRAIN, &term_save).unwrap();
    std::process::exit(0);*/
    (builtin::BUILTINS[2].2)(vec![""], &term_save);
}

fn exec_loop<'a>(term_save: &nix::sys::termios::Termios) {
    let mut recall_buf: Vec<String> = Vec::with_capacity(RECALL_BUF_LEN);
    let mut c_line = String::new();
    loop {
        display_prompt();
        // Yes, we want to panic on failed I/O flush.

        c_line = match read_line() {
            Ok(c) => { c },
            Err(e) => {
                println!("sh: {}", e);
                break;
            },
        };
        let l = c_line.clone();
        let args = split_line(l.as_str());

        recall_buf.push(c_line);
        exec_line(args, term_save);
    }
}

fn display_prompt() {
    print!("<{}> => ", ::std::env::current_dir().unwrap().display());
    io::stdout().flush().unwrap();
}


fn repaint(ret: &String, pos: usize) {
    print!("\x1b[{}D", pos);
    print!("\x1b[K");
    print!("{}", ret);
    if pos <= ret.len() {
        print!("\x1b[{}D", ret.len() - pos);
    }
    io::stdout().flush().unwrap();
}


fn read_line() -> Result<String, &'static str> {
    let mut ret = String::with_capacity(LINE_BUF_LEN);
    let mut pos: usize = 0;
    let mut esc_seq = false;
    let mut escape_seq = false;
    let mut quote_stack: Vec<char> = Vec::new();

    fn insert(ret: &mut String, pos: &mut usize, c: char) {
        ret.insert(*pos, c);
        *pos += 1;
        let len = ret.len();
        if *pos < len {
            repaint(&ret, *pos);
        }
    }

    
    for byte in io::stdin().bytes() {
        let byte = byte.unwrap();
        match byte {
            b'\\' => {
                print!("\\");
                io::stdout().flush().unwrap();
                if !escape_seq {
                    escape_seq = true;
                    continue;
                }
            },
            q @ b'"' | q @ b'\'' | q @ b'`' => {
                let q = q as char;
                insert(&mut ret, &mut pos, q);
                print!("{}", q);
                io::stdout().flush().unwrap();
                if !escape_seq {
                    if !quote_stack.is_empty() && quote_stack[quote_stack.len() - 1] == q {
                        quote_stack.pop();
                    } else if quote_stack.is_empty() {
                        quote_stack.push(q);
                    }
                }
            },
            b'\n' => { 
                if escape_seq || quote_stack.len() > 0 {
                    print!("\n> ");
                    io::stdout().flush().unwrap();
                } else {
                    println!();     
                    break; 
                }
            },
            b'\x1b' => { esc_seq = true; continue; },
            b'\x7f' => {
                if pos > 0 {
                    ret.remove(pos - 1);
                    repaint(&ret, pos);
                    pos -= 1;
                    io::stdout().flush().unwrap();
                }
            },
            b'[' => {
                if esc_seq { continue; }
                else { insert(&mut ret, &mut pos, '['); }
            },
            b'D' => {
                if esc_seq {
                    if pos > 0 {
                       print!("\x1b[1D");
                       io::stdout().flush().unwrap();
                       pos -= 1;
                    }
                } else { insert(&mut ret, &mut pos, 'D'); }
            },
            b'C' => {
                if esc_seq {
                    if pos < ret.len() {
                        print!("\x1b[1C");
                        io::stdout().flush().unwrap();
                        pos += 1;
                    }
                } else { insert(&mut ret, &mut pos, 'C'); }
            },
            3 => { // C-c
                println!("C-c");
                ret.clear();
                break;
            },
            _ => { 
                print!("{}", byte as char);
                io::stdout().flush().unwrap();
                insert(&mut ret, &mut pos, byte as char);
            },
        }
        esc_seq = false;
        escape_seq = false;
    }
    Ok(ret)
}

/*fn split_line<'a>(line: &'a str) -> Vec<&'a str> {
    line.split_whitespace().collect()
}*/

fn split_line(line: &str) -> Vec<String> {
    let mut ret = Vec::new();
    let mut escaping = false;
    let mut quote_stack = Vec::with_capacity(1);
    let mut c_arg = String::new();
    for byte in line.bytes() {
        match byte {
            b'"' | b'\'' | b'`' => {
                if quote_stack.is_empty() {
                    quote_stack.push(byte as char);
                } else if quote_stack[quote_stack.len() - 1] == byte as char {
                    quote_stack.pop();
                } else {
                    c_arg.push(byte as char);
                }
            },
            b'\\' => {
                if escaping {
                    c_arg.push('\\');
                } else {
                    escaping = true;
                    continue;
                }
            },
            b' ' | b'\t' => {
                if escaping || !quote_stack.is_empty() {
                    c_arg.push(byte as char)
                } else {
                    ret.push(c_arg);
                    c_arg.clear();
                }
            },
            _ => {
                c_arg.push(byte as char);
            }
        }
        escaping = false;
    }
    ret
}

fn launch_subprocess(name: &str, args: Vec<&str>) {
    let status = Command::new(&name).args(&args).status();
    let status = match status {
        Ok(s) => s,
        Err(e) => { 
            println!("Failed to launch subprocess. {}", e);
            return;
        },
    };
    if !status.success() {
        println!("Process {} exited with status {}", name, status);
    }
}

fn exec_line(args: Vec<String>, term_save: &nix::sys::termios::Termios) {
    if args.len() < 1 {
        return;
    }

    let args: Vec<&str> = args.into_iter().map(|a: String| { a.as_str() }).collect();
    for bn in builtin::BUILTINS.into_iter() {
        if (*bn).0 == args[0] {
            (*bn).2((&args[1..]).to_vec(), term_save);
            return;
            
        }
    }
    
    launch_subprocess(args[0], (&args[1..]).to_vec());
}
