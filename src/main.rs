extern crate nix;

use std::io;
use std::process::Command;
use std::io::Write;
use std::io::Read;
use nix::sys::termios;

const LINE_BUF_LEN: usize = 50;


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

fn exec_loop(term_save: &nix::sys::termios::Termios) {
    loop {
        let l: String;
        display_prompt();
        // Yes, we want to panic on failed I/O flush.
        let args = split_line(match read_line() {
            Ok(c) => { l = c; l.as_str() },
            Err(e) => { 
                println!("sh: {}", e);
                break;
            },
        });
        //println!("{}", read_line().unwrap());
        exec_line(args, term_save);
    }
}

fn display_prompt() {
    print!("<{}> => ", ::std::env::current_dir().unwrap().display());
    io::stdout().flush().unwrap();
}

/*fn read_line() -> Result<String, io::Error> {
    let mut ret = String::new(); 
    io::stdin().read_line(&mut ret)?;
    Ok(ret)
}*/

fn read_line() -> Result<String, &'static str> {
    let mut ret = String::with_capacity(LINE_BUF_LEN);
    let mut pos: usize = 0;
    let mut esc_seq = false;
    fn insert(ret: &mut String, pos: &mut usize, c: char) {
        ret.insert(*pos, c);
        *pos += 1;
    }
    for byte in io::stdin().bytes() {
        let byte = byte.unwrap();
        match byte {
            b'\n' => { 
                println!();     
                break; 
            },
            b'\x1b' => { esc_seq = true; continue; },
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
                    if pos < ret.len() - 1 {
                        print!("\x1b[1C");
                        io::stdout().flush().unwrap();
                        pos += 1;
                    }
                } else { insert(&mut ret, &mut pos, 'C'); }
            },
            3 => { // C-c
                return Err("Terminating.");
            }
            _ => { print!("{}", byte as char);
                io::stdout().flush().unwrap();
                insert(&mut ret, &mut pos, byte as char);
            },
        }
        esc_seq = false;
    }
    Ok(ret)
}

fn split_line<'a>(line: &'a str) -> Vec<&'a str> {
    line.split_whitespace().collect()
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

fn exec_line(args: Vec<&str>, term_save: &nix::sys::termios::Termios) {
    if args.len() < 1 {
        return;
    }

    for bn in builtin::BUILTINS.into_iter() {
        if (*bn).0 == args[0] {
            (*bn).2((&args[1..]).to_vec(), term_save);
            return;
            
        }
    }
    
    launch_subprocess(args[0], (&args[1..]).to_vec());
}
