//只有被mod声明的模块才会被编译器编译，且mod声明的模块必须在当前文件所在目录下有一个同名的.rs文件或者一个同名的目录，并且该目录下有一个mod.rs文件
mod file_read;
mod data_struct;
mod content_check;
mod config;

fn main() {
    config::init();
    println!("Hello, world!");
}
