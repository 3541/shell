use nix;
use std::env;
use std::path::Path;


// TODO design builtin structure such that iteration over the whole array (worst case O(n)) is not
// necessary for each command entry.
pub const BUILTINS: [(&'static str, &'static str, fn(Vec<&str>, &nix::sys::termios::Termios)); 3] = [
    (
        "cd", 
        "cd: Change directory\n\
        Usage:\n\
        \t  cd [path]\n\
        If invoked without a path, directory will be changed to the home directory.",
        cd
    ),
    (
        "help", 
        "help: Help\n
        Usage:\n\
        \t  help [command]\n\
        Gives instructional help either for the shell or a command, if invoked with an argument",
        help
    ),
    (
        "exit", 
        "exit: Exit shell\n\
        Usage:\n\
        \t  exit",
        exit
    )
];

#[allow(unused_variables)]
fn cd(args: Vec<&str>, term_save: &nix::sys::termios::Termios) {
    if args.len() < 1 {
        println!("Expected an argument to cd.");
    } else {
        match env::set_current_dir(&(Path::new(args[0]))) {
            Ok(_) => {},
            Err(e) => { println!("Failed to change directory. {}", e); },
        }
    }
}

#[allow(unused_variables)]
fn help(args: Vec<&str>, term_save: &nix::sys::termios::Termios) {
    if args.len() < 1 {
        println!("A shell.");
        println!("Interact in the obvious ways.");
        println!("\nThe following are shell builtins:");
        println!("\nUse man for information on non-builtin programs.");
        for bn in BUILTINS.into_iter() {
            println!("\t{}", (*bn).0);
        }
    } else {
        for bn in BUILTINS.into_iter() {
            if (*bn).0 == args[0] {
                println!("{}", (*bn).1);
            }
        }
    }

}

use nix::sys::termios;
fn exit(args: Vec<&str>, term_save: &nix::sys::termios::Termios) {
    if args.len() > 0 {
        println!("Exiting ({}).", args[0]);
    }
    termios::tcsetattr(0, termios::TCSADRAIN, term_save).unwrap();
    ::std::process::exit(0);
}
