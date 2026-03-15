//只有被mod声明的模块才会被编译器编译，且mod声明的模块必须在当前文件所在目录下有一个同名的.rs文件或者一个同名的目录，并且该目录下有一个mod.rs文件
mod file_read;
mod data_struct;
mod content_check;
mod config;
mod show;

use std::path; 
//clap库能够自动生成帮助信息
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
    /// （short对应-p,long对应--path）
    #[arg(short, long)]
    path: String,
    //检索内容
    #[arg(short, long)]
    query:String,

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

    let read_data_vec: data_struct::DataStructVec;
    let mut check_result_container: data_struct::FilePageContainer;


    //获取文件数据
    match check_path_type(path_obj.as_path()) {

        PathType::Directory=>{
            read_data_vec = file_read::read_files_in_directory(path_obj.as_path()).unwrap();
            
        },
        
        PathType::File =>{
            read_data_vec = file_read::read_one_file(path_obj.as_path()).unwrap();
        },
        PathType::Unsupported=> {
            panic!("当前路径格式不支持，合法路径包括可读文件以及目录")
        }

    }
    //注意此处引用切片与vec之间的转换技巧,先用iter和collect将内部数据转换成指针，之后再用引用转换成切片
    let dsp_refs: Vec<&data_struct::DataStructPackage> = read_data_vec.data_structs.iter().collect();
    check_result_container = content_check::query_data_struct(&dsp_refs,&args.query).unwrap();

    if let Err(e) = show::Visualizer::show(&mut check_result_container) {
        eprintln!("展示错误: {}", e);
        std::process::exit(1);
    }


}


//还是尽量利用AI的低成本代码，尽量让单元测试都覆盖
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir; // 需要在 dev-dependencies 中添加 tempfile

    #[test]
    fn test_get_config_success() {
        // 1. 确保初始化（如果 CONFIG 还没初始化过）
        // 注意：全局变量在测试进程中是共享的，可能已被其他测试初始化
        config::init(); 

        // 2. 调用目标方法
        let _config = config::get_config();
    }

    #[test]
    fn test_check_path_type() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        File::create(&file_path).unwrap();

        // 测试目录
        assert!(matches!(check_path_type(dir.path()), PathType::Directory));
        // 测试文件
        assert!(matches!(check_path_type(&file_path), PathType::File));
        // 测试不存在的路径
        let ghost_path = path::Path::new("i_dont_exist");
        assert!(matches!(check_path_type(ghost_path), PathType::Unsupported));
    }
}