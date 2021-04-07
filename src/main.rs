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

/// Resolve a duplicate
fn duplicate_resolver(duplicates: &Vec<PathBuf>, destructive: bool) {

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
                let _ = fs::remove_file(duplicate);
                let _ = fs::hard_link(source, duplicate);
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
            duplicate_resolver(entry, opts.relink);
        }
    }


}