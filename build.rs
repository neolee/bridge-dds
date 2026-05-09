fn main() {
    println!("cargo:rustc-link-search=native=engine/dds/lib");
    println!("cargo:rustc-link-lib=static=dds");
    println!("cargo:rustc-link-lib=dylib=c++");
}
