extern crate toml;

#[macro_export]
macro_rules! refstruct {
    ($e: tt) => { concat!(env!("OUT_DIR"), "/ref_struct/", file!(), "/", line!()) };
}

use std::path::{Path, PathBuf};
use std::{io, fs, env};

pub struct Scanner {}

impl Scanner {
    pub fn process_src() -> io::Result<()> {
        let mut in_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        in_dir.push("src");
        let mut out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        out_dir.push("ref_struct");
        out_dir.push("src");
        Scanner::process_dir(&in_dir, &out_dir, true)
    }

    pub fn process_dir(in_dir: &Path, out_dir: &Path, recurse: bool) -> io::Result<()> {
        let files = try!(fs::read_dir(in_dir));
        for entry in files {
            let entry = try!(entry);
            let ft = try!(entry.file_type());
            if recurse && ft.is_dir() {
                let mut odir = PathBuf::from(out_dir);
                odir.push(entry.file_name());
                try!(Scanner::process_dir(&entry.path(), &odir, true));
            }
            if ft.is_file() {
                let mut odir = PathBuf::from(out_dir);
                odir.push(entry.file_name());
                try!(Scanner::process_file(&entry.path(), &odir));
            }
        }
        Ok(())
    }

    pub fn process_file(in_file: &Path, out_dir: &Path) -> io::Result<()> {
        use std::io::{Write, BufRead};
        // panic!("== From == {} == to == {}", in_file.to_string_lossy(), out_dir.to_string_lossy());
        let file = try!(fs::File::open(in_file));
        let buf = io::BufReader::new(file);
        let mut startline = None;
        let mut s = String::new();
        let mut errstr = String::new();
        for (lineno, line) in buf.lines().enumerate() {

            let l = try!(line);
            if startline.is_none() {
                if l.find("ref_struct!").is_some() { startline = Some(lineno+1) };
                continue;
            }

            if l.find("\"#").is_none() {
                s.push_str(&l);
                s.push('\n');
                continue;
            }

            errstr = format!("ref_struct macro at {}:{}", in_file.to_string_lossy(), startline.unwrap());

            let sw = try!(StructWriter::from_toml(&s).map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("{} {}", errstr, e))));
            let contents = sw.write_struct();
            s = String::new();

            try!(fs::create_dir_all(&out_dir));
            let mut ofile = PathBuf::from(out_dir);
            ofile.push(&format!("{}", startline.unwrap()));
            startline = None;
            let mut outf = try!(fs::File::create(&ofile));
            try!(write!(outf, "{}", contents));
        }
        if startline.is_some() { Err(io::Error::new(io::ErrorKind::Other,
            format!("{} is not a raw string literal", errstr))) } 
        else { Ok(()) }
    }
}


pub struct StructWriter {
    name: String,
    module: String,
    lt: String,
    imports: Vec<String>,
    fields: Vec<(String, String)>,
}

impl StructWriter {
    pub fn from_toml(input: &str) -> Result<StructWriter, String> {
        let mut toml_parser = toml::Parser::new(input);
        let v = try!(toml_parser.parse().ok_or_else(|| format!("is not valid TOML: {:?}", toml_parser.errors)));
        // panic!("{:?}", v);
        let name = try!(v.get("name").and_then(|v| v.as_str()).ok_or(" - missing or invalid 'name' key"));
        let module = v.get("module").and_then(|v| v.as_str()).map(|s| s.into()).unwrap_or_else(|| name.to_lowercase());
        let lt = format!("'{}", module);

        let imports = v.get("use").and_then(|v| v.as_slice()).unwrap_or(&[]);
        let imports: Vec<String> = imports.iter().filter_map(|s| s.as_str()).map(|s| String::from(s)).collect();

        let mut fields: Vec<(String, String)> = vec!();
        let vfields = try!(v.get("fields").and_then(|v| v.as_slice()).ok_or(" - missing or invalid 'fields' key"));
        for f in vfields {
            let f = try!(f.as_slice().ok_or(" - all fields are not arrays"));
            let k = try!(f.get(0).and_then(|k| k.as_str()).ok_or(" - field subarray must be two strings"));
            let v = try!(f.get(1).and_then(|k| k.as_str()).ok_or(" - field subarray must be two strings"));
            fields.push((k.into(), v.into()));
            // let ty = try!(ty.as_str().ok_or_else(|| format!(" - field '{}' has invalid type", key)));
            // fields.push((key.clone(), ty.into()));
        }

        Ok(StructWriter { name: name.into(), module: module, fields: fields, lt: lt, imports: imports })
    }
/*
    fn write_getter(&self, k: &str, v: &str, outside_mod: bool) -> String {
        let indent = if outside_mod { "    " } else { "        " };
        let modprefix = if outside_mod { format!("{}::", self.module) } else { String::new() };
        format!("{}#[allow(dead_code)]\n{}pub fn {}<{}>(&{} self) -> &{} {} {{ unsafe {{ {}Ptr::{}(&self.0) }} }}\n",
            indent, indent, k, self.lt, self.lt, self.lt, v.replace("'_", &self.lt), modprefix, k)
    }
*/
    fn write_ptr(&self, ptr_ty: &str, mutstr: &str, ptrstr: &str) -> String {
        let mut s = format!(r#"
    struct {} {{}}
    impl {} {{"#, ptr_ty, ptr_ty);
        for &(ref k, ref v) in &self.fields {
            s.push_str(&format!(r#"
        unsafe fn {}<{}>(a: & {} {} Box<[u8]>) -> & {} {} {} {{
            debug_assert_eq!(::std::mem::size_of::<Raw<'static>>(), a.len());
            &{} (&{} *(&{} a[0] as *{} _ as *{} Raw<{}>)).{}
        }}
"#,
                k, self.lt, self.lt, mutstr, self.lt, mutstr, v.replace("'_", &self.lt),
                mutstr, mutstr, mutstr, ptrstr, ptrstr, self.lt, k));
        }
        s.push_str("    }\n");
        s
    }


    fn write_ptrwrite(container: &str, k: &str, consume: &str) -> String {
        format!("unsafe {{ ::std::ptr::write(Ptr::{}(& {}) as *const _ as *mut _, {}) }}",
            k, container, consume) 
    }

    fn write_step(&self, step: usize) -> String {
        let mut s = format!(r#"
    #[derive(Debug)]
    pub struct Step{}(Box<[u8]>);
    impl Step{} {{"#, step, step);

        // First init
        if step == 1 { s.push_str(&format!(r#"
        pub fn new(p: {}) -> Step1 {{
            let v = vec!(0; ::std::mem::size_of::<Raw<'static>>());
            let r = v.into_boxed_slice();
            {};
            Step1(r)
        }}"#, self.fields[0].1, Self::write_ptrwrite("r", &self.fields[0].0, "p")));
        }

        // Final build
        if step == self.fields.len() {
            s.push_str(&format!(r#"
        pub fn build(self) -> super::{} {{ super::{}(self) }}"#, self.name, self.name));
        }

        // To next step
        if step < self.fields.len() {
            let (ref k, ref v) = self.fields[step];
            s.push_str(&format!(r#"
        pub fn {}<F>(mut self, f: F) -> Step{}
            where F: for<{}> FnOnce(&{} Self) -> {}
        {{
            {{
                let r = f(&self);
                {};
            }}
            let b = ::std::mem::replace(&mut self.0, Box::new([]));
            ::std::mem::forget(self);
            Step{}(b)
        }}"#, k, step+1, self.lt, self.lt, v.replace("'_", &self.lt),
            Self::write_ptrwrite("self.0", k, "r"), step+1));
        }

        // Getters
        for &(ref k, ref v) in self.fields.iter().take(step) {
            s.push_str(&format!(r#"
        pub fn {}<{}>(&{} self) -> &{} {} {{
            unsafe {{ Ptr::{}(&self.0) }}
        }}"#, k, self.lt, self.lt, self.lt, v.replace("'_", &self.lt), k))
        }

        s.push_str("\n    }\n");

        // Drop
        s.push_str(&format!(r#"
    impl Drop for Step{} {{
        fn drop(&mut self) {{
            let _ = unsafe {{ ::std::ptr::read(Ptr::{}(&self.0)) }};{}
        }}
    }}"#, step, self.fields[step-1].0, if step > 1 {
            format!(r#"
            let _ = Step{}(::std::mem::replace(&mut self.0, Box::new([])));"#, step-1) } else { "".into() } 
        ));

        s
    }

    fn write_outer(&self) -> String {
        let mut s = format!(r#"
#[derive(Debug)]
pub struct {}({}::Step{});
impl {} {{
    #[allow(dead_code)]
    #[inline]
    pub fn new(a: {}) -> {}::Step1 {{ {}::Step1::new(a) }}"#,
            self.name, self.module, self.fields.len(), self.name, self.fields[0].1, self.module, self.module);
        for &(ref k, ref v) in &self.fields {
            s.push_str(&format!(r#"

    #[allow(dead_code)]
    #[inline]
    pub fn {}<{}>(&{} self) -> &{} {} {{ self.0.{}() }}"#,
            k, self.lt, self.lt, self.lt, v.replace("'_", &self.lt), k));
        }
        s.push_str("\n}\n");
        s
    }

    pub fn write_struct(&self) -> String {
        let mut s = String::new();

        s.push_str(&format!("#[allow(dead_code)]\npub mod {} {{\n", self.module));

        // Write use
        for i in &self.imports {
            s.push_str(&format!("\n    use {};", i));
        }

        // Write raw repr struct
        s.push_str("\n");
        s.push_str(&format!("    struct Raw<{}> {{\n", self.lt));
        for &(ref k, ref v) in &self.fields {
            s.push_str(&format!("        {}: {},\n", k, v.replace("'_", &self.lt)));
        }
        s.push_str("    }\n");

        // Write ptrs
        s.push_str(&self.write_ptr("Ptr", "", "const"));
        // s.push_str(&self.write_ptr("PtrMut", "mut", "mut"));

        // Write all steps
        for i in 0..self.fields.len() { s.push_str(&self.write_step(i+1)) }; 

        // End of module
        s.push_str("\n}\n");

        // Make final struct outside module
        s.push_str(&self.write_outer());
        s
    }
}
