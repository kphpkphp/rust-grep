//只有被mod声明的模块才会被编译器编译，且mod声明的模块必须在当前文件所在目录下有一个同名的.rs文件或者一个同名的目录，并且该目录下有一个mod.rs文件
mod file_read;
mod data_struct;
mod content_check;
mod config;
mod show;

use std::path; 
use clap::Parser;

#[derive(Debug)]
enum PathType{
    File,
    Directory,
    Unsupported,
}

//这里需要注意，clap库的Parser功能需要在toml文件里配置才能生效，不是自动启用的
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 路径
    #[arg(short, long)]
    path: String,

}

fn check_path_type(path:&path::Path)->PathType{

    if path.is_dir(){
        return PathType::Directory;
    }
    else if path.is_file() {
        return PathType::File;
    }

    PathType::Unsupported

}

//获取终端传入的路径，解析路径，执行读取
//读取到数据之后，调用show模块展示

fn main() {
    config::init();

    let args = Args::parse();
    let path_str = args.path;
    let path_obj = path::Path::new(&path_str).to_path_buf();

    let mut read_data_vec:data_struct::DataStructVec;

    match check_path_type(path_obj.as_path()) {

        PathType::Directory=>{
            read_data_vec = file_read::read_one_file(path_obj.as_path()).unwrap();
        },
        
        PathType::File =>{
            read_data_vec = file_read::read_files_in_directory(path_obj.as_path()).unwrap();
        },
        PathType::Unsupported=> {
            panic!("当前路径格式不支持，合法路径包括可读文件以及目录")
        }

    }



}
