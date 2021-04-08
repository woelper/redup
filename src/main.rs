use walkdir::WalkDir;
use std::{collections::HashMap, hash::Hasher, io, path::{Path, PathBuf}};
use std::fs::File;
use twox_hash::XxHash64;
use gumdrop::Options;
use std::fs;


#[derive(Debug, Options)]
struct MyOptions {

    #[options(help = "Root directory to scan")]
    root: String,


    #[options(help = "Enable destructive relinking")]
    relink: bool,

    #[options(help = "Perform soft linking")]
    softlink: bool,

    #[options(help = "Prefer originals if containing this string")]
    resolve_filter: Option<String>,

}


struct HashWriter<T: Hasher>(T);

impl<T: Hasher> io::Write for HashWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.write(buf).map(|_| ())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Produce a hash from a file
fn hash_file(file: &Path) -> u64{
    let mut f = File::open(file).expect("Unable to open file");
    let hasher = XxHash64::with_seed(0);
    let mut hw = HashWriter(hasher);
    io::copy(&mut f, &mut hw).expect("Unable to copy data");
    let hasher = hw.0;
    hasher.finish()
}

/// Link a file, optionally as soft link.
fn link(src: &Path, dest: &Path, soft: bool) -> io::Result<()> {
    if soft {
        #[cfg(target_os = "windows")]
        {
            std::os::windows::fs::symlink_file(src, dest)
        }
        #[cfg(target_family = "unix")]
        {
            std::os::unix::fs::symlink(src, dest)
        }   
    }
    else {
        fs::hard_link(src, dest)
    }
}

fn safe_link(src: &Path, dest: &Path, soft: bool) {
    let dest_backup = dest.with_extension("rdup");
    let _ = fs::rename(dest, &dest_backup);
    
    match link(src, dest, soft) {
        Ok(_) => {
            let _ = fs::remove_file(dest_backup);
        },
        Err(e) => {
            // Rename back, link failed
            let _ = fs::rename(dest_backup, dest);
            eprintln!("Could not link {:?}", e);
        }
    }
}

/// Resolve a duplicate
fn duplicate_resolver(duplicates: &Vec<PathBuf>, destructive: bool, softlink: bool) {

    // Safeguard against having no duplicates
    if duplicates.len() < 2 {
        return
    }

    if let Some(source) = duplicates.first() {
        let dest = &duplicates[1..];
        if destructive {
            println!("Relinking to {:?}:", source);

            for duplicate in dest {
                println!("\tLinking {:?}", duplicate);
                safe_link(&source, &duplicate, softlink);
            }
        }
    }
}


fn main() {


    let opts = MyOptions::parse_args_or_exit(gumdrop::ParsingStyle::AllOptions);
    if opts.root == "" {
        eprintln!("You must supply a --root parameter");
        return;
    }
    
    let mut db: HashMap<u64, Vec<PathBuf>> = HashMap::new();


    for entry in WalkDir::new(opts.root).into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|f| !f.path_is_symlink()) {
        

            // TODO: skip file if it's any type of link
            
            if entry.path().read_link().is_ok() {
                continue;
            }
            let hash = hash_file(entry.path());
            let e = db.entry(hash).or_default();
            e.push(entry.path().to_owned());
    }
    // at this point we have all files ordered by hash, with filenames. Everything with more than one entry is a duplicate.

    for entry in db.values() {
        if entry.len() > 1 {
            duplicate_resolver(entry, opts.relink, opts.softlink);
        }
    }


}