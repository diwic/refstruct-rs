extern crate toml;

/// The refstruct macro. Use like this:
///
/// ```ignore
/// include!(refstruct!(r#"
/// // TOML content
/// "#);
/// ```
#[macro_export]
macro_rules! refstruct {
    ($e: tt) => { concat!(env!("OUT_DIR"), "/refstruct/", file!(), "/", line!()) };
}

use std::path::{Path, PathBuf};
use std::{io, fs, env};

pub struct Scanner {}

impl Scanner {
    pub fn process_src() -> io::Result<()> {
        let mut in_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        in_dir.push("src");
        let mut out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        out_dir.push("refstruct");
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
                if l.find("refstruct!").is_some() { startline = Some(lineno+1) };
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
    sprefix: String,
    imports: Vec<String>,
    fields: Vec<(String, String)>,
}

impl StructWriter {
    pub fn from_toml(input: &str) -> Result<StructWriter, String> {
        let mut toml_parser = toml::Parser::new(input);
        let v = try!(toml_parser.parse().ok_or_else(|| format!("is not valid TOML: {:?}", toml_parser.errors)));

        let name = try!(v.get("name").and_then(|v| v.as_str()).ok_or(" - missing or invalid 'name' key"));
        if name == "" { return Err(" - name cannot be empty".into()) }

        let namespace = (if let Some(ns) = v.get("namespace") {
            try!(ns.as_str().ok_or(" - namespace is not a string"))
        } else { &*name }).to_lowercase();
        if namespace == "" { return Err(" - name cannot be empty".into()) }

        let sprefix = format!("{}{}", namespace.to_uppercase().chars().next().unwrap(),
            namespace.chars().skip(1).collect::<String>());

        let module = (if let Some(ns) = v.get("module") {
            try!(ns.as_str().ok_or(" - module is not a string"))
        } else { &*namespace }).to_string();

        let lt = format!("'{}", if let Some(ns) = v.get("lifetime") {
            try!(ns.as_str().ok_or(" - lifetime is not a string"))
        } else { &*namespace });

        let imports = v.get("use").and_then(|v| v.as_slice()).unwrap_or(&[]);
        let imports: Vec<String> = imports.iter().filter_map(|s| s.as_str()).map(|s| String::from(s)).collect();

        let mut fields: Vec<(String, String)> = vec!();
        let vfields = try!(v.get("fields").and_then(|v| v.as_slice()).ok_or(" - missing or invalid 'fields' key"));
        for f in vfields {
            let f = try!(f.as_slice().ok_or(" - all fields are not arrays"));
            let k = try!(f.get(0).and_then(|k| k.as_str()).ok_or(" - field subarray must be two strings"));
            let v = try!(f.get(1).and_then(|k| k.as_str()).ok_or(" - field subarray must be two strings"));
            if k == "new" || k == "build" { return Err(" - fields cannot be named 'new' or 'build'".into()) }
            fields.push((k.into(), v.into()));
        }

        Ok(StructWriter { name: name.into(), module: module, fields: fields, lt: lt,
            sprefix: sprefix, imports: imports })
    }

    fn write_ptr(&self, ptr_ty: &str, mutstr: &str, ptrstr: &str) -> String {
        let mut s = format!(r#"
    struct {} {{}}
    impl {} {{"#, ptr_ty, ptr_ty);
        for &(ref k, ref v) in &self.fields {
            s.push_str(&format!(r#"
        unsafe fn {}<{}>(a: & {} {} Box<[u8]>) -> & {} {} {} {{
            debug_assert_eq!(::std::mem::size_of::<{}Raw<'static>>(), a.len());
            &{} (&{} *(&{} a[0] as *{} _ as *{} {}Raw<{}>)).{}
        }}
"#,
                k, self.lt, self.lt, mutstr, self.lt, mutstr, v.replace("'_", &self.lt),
                self.sprefix, mutstr, mutstr, mutstr, ptrstr, ptrstr, self.sprefix, self.lt, k));
        }
        s.push_str("    }\n");
        s
    }

    fn write_ptrwrite(&self, container: &str, k: &str, consume: &str) -> String {
        format!("unsafe {{ ::std::ptr::write(&mut (&mut *(&mut {}[0] as *mut _ as *mut {}Raw<'static>)).{}, {}) }}",
            container, self.sprefix, k, consume) 
    }

    fn write_step(&self, step: usize) -> String {
        let stepstr = format!("{}Step{}", self.sprefix, step);
        let mut s = format!(r#"
    #[derive(Debug)]
    pub struct {}(Box<[u8]>, ::std::marker::PhantomData<{}Raw<'static>>);
    impl {} {{"#, stepstr, self.sprefix, stepstr);

        // First init
        if step == 1 { s.push_str(&format!(r#"
        pub fn new(p: {}) -> {} {{
            let v = vec!(0; ::std::mem::size_of::<{}Raw<'static>>());
            let mut r = v.into_boxed_slice();
            {};
            {}(r, ::std::marker::PhantomData)
        }}"#, self.fields[0].1, stepstr, self.sprefix, self.write_ptrwrite("r", &self.fields[0].0, "p"), stepstr));
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
        pub fn {}<F>(mut self, f: F) -> {}Step{}
            where F: for<{}> FnOnce(&{} Self) -> {}
        {{
            let r: {} = unsafe {{ ::std::mem::transmute(f(&self)) }};
            let mut b = ::std::mem::replace(&mut self.0, Box::new([]));
            ::std::mem::forget(self);
            {};
            {}Step{}(b, ::std::marker::PhantomData)
        }}"#, k, self.sprefix, step+1, self.lt, self.lt, v.replace("'_", &self.lt),
            v.replace("'_", "'static"), self.write_ptrwrite("b", k, "r"), self.sprefix, step+1));
        }

        // Getters
        for &(ref k, ref v) in self.fields.iter().take(step) {
            s.push_str(&format!(r#"
        pub fn {}<{}>(&{} self) -> &{} {} {{
            unsafe {{ {}Ptr::{}(&self.0) }}
        }}"#, k, self.lt, self.lt, self.lt, v.replace("'_", &self.lt), self.sprefix, k))
        }

        s.push_str("\n    }\n");

        // Drop
        s.push_str(&format!(r#"
    impl Drop for {} {{
        fn drop(&mut self) {{
            let _ = unsafe {{ ::std::ptr::read({}Ptr::{}(&self.0)) }};{}
        }}
    }}"#, stepstr, self.sprefix, self.fields[step-1].0, if step > 1 {
            format!(r#"
            let _ = {}Step{}(::std::mem::replace(&mut self.0, Box::new([])), ::std::marker::PhantomData);"#,
                self.sprefix, step-1)
            } else { "".into() } 
        ));

        s
    }

    fn write_outer(&self) -> String {
        let stepstr = format!("{}::{}Step", self.module, self.sprefix);
        let mut s = format!(r#"
#[derive(Debug)]
pub struct {}({}{});
impl {} {{
    #[allow(dead_code)]
    #[inline]
    pub fn new(a: {}) -> {}1 {{ {}1::new(a) }}"#,
            self.name, stepstr, self.fields.len(), self.name, self.fields[0].1, stepstr, stepstr);
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
        s.push_str(&format!("    struct {}Raw<{}> {{\n", self.sprefix, self.lt));
        for &(ref k, ref v) in &self.fields {
            s.push_str(&format!("        {}: {},\n", k, v.replace("'_", &self.lt)));
        }
        s.push_str("    }\n");

        // Write ptrs
        s.push_str(&self.write_ptr(&format!("{}Ptr", self.sprefix), "", "const"));
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
