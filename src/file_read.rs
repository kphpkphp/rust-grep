/*该模块实现文件读取功能,以及从目录中批量读取可用文件*/

use crate::data_struct::{DataStruct, DataStructPackage, DataStructVec};
use std::f32::consts::E;
use std::fs;
use std::path::{Path,PathBuf};
use std::str::FromStr;
use std::io::{BufRead};
use thiserror::Error;
// use anyhow::Result;


enum ValidFormats {
    Txt,
    Csv,
    Json,
    Log,
    Dat,
}

//实现从文件扩展名到 ValidFormats 的转换
impl FromStr for ValidFormats {
    type Err = ();

    fn from_str(ext: &str) -> Result<Self, Self::Err> {
        match ext.to_lowercase().as_str() {
            "txt" => Ok(ValidFormats::Txt),
            "csv" => Ok(ValidFormats::Csv),
            "json" => Ok(ValidFormats::Json),
            "log" => Ok(ValidFormats::Log),
            "dat" => Ok(ValidFormats::Dat),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub enum FileReadError {
    NotFound,
    FileTooLarge,
    UnsupportedFormat,
    NoExtension,
    ReadError(String),  
    IOError(std::io::Error),
}


#[derive(Error,Debug)]
pub enum PathReadError {
    #[error("路径不存在")]
    NotFound,
    #[error("不是一个目录")]
    NotADirectory,
    #[error("文件系统原生错误: {0}")]
    IOError(#[from] std::io::Error),
}


//检查文件是否合法,
fn check_file(file_path: &str) -> Result<PathBuf, FileReadError> {
    //为防止过多的堆分配，一开始仅通过只读模式进行观察和检查
    let path_ref = Path::new(file_path);

    //检查文件是否存在且是一个文件，而不是目录或其他类型的路径
    if !path_ref.exists() || !path_ref.is_file() {
        return Err(FileReadError::NotFound);
    }

    //这里仅获取metadata，map_err 将 std::io::Error 转换为 FileReadError::IOError,Ok则什么都不做。即，如果发生错误则返回 FileReadError::IOError，否则继续执行
    //注意，metadata()会导致所有权移动
    let metadata = fs::metadata(path_ref).map_err(FileReadError::IOError)?;

    if metadata.len() > 10 * 1024 * 1024 {
        return Err(FileReadError::FileTooLarge);
    }

    //.extension() 返回 Option<OsStr>（可能有没有扩展名的文件，所以是Option），and_then 将 Option<OsStr> 转换为 Option<&str>，ok_or 将 Option<&str> 转换为 Result<&str, FileReadError::NoExtension>

    // let ext_str = path_ref.extension()
    //     //and_then 是 Option 的一个方法，用于链式调用。当 Option 是 Some 时，and_then 会调用提供的闭包函数，并将 Some 中的值作为参数传递给该函数。如果 Option 是 None，则 and_then 直接返回 None，不会调用闭包函数。
    //     .and_then(|ext| ext.to_str())
    //     //ok_or 是 Option 的一个方法，用于将 Option 转换为 Result。当 Option 是 Some 时，ok_or 会返回 Ok(Some 中的值)。当 Option 是 None 时，ok_or 会返回 Err(提供的错误值)。? 运算符用于简化错误处理，如果 Result 是 Err，则会立即返回该错误，否则继续执行后续代码。
    //     .ok_or(FileReadError::NoExtension)?;

    //map_err 是 Result 的一个方法，用于将 Result 中的错误值转换为另一种类型。当 Result 是 Ok 时，map_err 会返回 Ok(原来的值)。当 Result 是 Err 时，map_err 会调用提供的闭包函数，并将 Err 中的错误值作为参数传递给该函数，然后返回 Err(闭包函数的返回值)。这里的闭包函数参数直接忽略了
    // ValidFormats::from_str(ext_str).map_err(|_| FileReadError::UnsupportedFormat)
    Ok(path_ref.to_path_buf())
}


pub fn read_file_to_data_struct(file_path: &str) -> Result<DataStructPackage, FileReadError> {

    let path_buf = check_file(file_path)?;

    // 读取文件内容并存储在 DataStruct 中的逻辑
    let mut data_struct = DataStruct {
        values: Vec::new(),
    };

    let file = std::fs::File::open(file_path).map_err(FileReadError::IOError)?;
    let reader = std::io::BufReader::new(file);

    for line in reader.lines() {
        let line = line.map_err(|e|FileReadError::ReadError(e.to_string()))?;
        data_struct.values.push(line);
    }

    let data_package = DataStructPackage {
        data_status: "success".to_string(),
        data_path: path_buf,
        data_struct,
    };

    Ok(data_package)

}


fn check_path_valid(path: &str) -> Result<(), PathReadError> {

    let path = Path::new(path);

    if !path.exists() {
        return Err(PathReadError::NotFound);
    }

    if !path.is_dir() {
        return Err(PathReadError::NotADirectory);
    }

    Ok(())
}

pub fn read_files_in_path(path: &str) -> Result<DataStructVec, PathReadError> {

    check_path_valid(path)?;

    let mut read_vec = DataStructVec {
        directory_path: path.to_string(),
        data_structs: Vec::new(),
    };

    // 1. fs::read_dir 可能会因为权限等问题失败
    let entries = fs::read_dir(path).map_err(PathReadError::IOError)?;

    for entry in entries {
        // 2. 迭代过程中的 entry 也可能出错
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Failed to read directory entry: {:?}", e);
                continue; // 跳过错误的 entry
            }
        };

        let file_path: std::path::PathBuf = entry.path();
        
        if file_path.is_file() {
            // 尽量避免 unwrap()，这里转换路径为字符串用于函数调用
            // let path_str = file_path.to_str().unwrap_or("<Invalid Unicode Path>");

            match read_file_to_data_struct(path_str) {
                Ok(data_package) => {
                    read_vec.data_structs.push(data_package);
                }
                Err(e) => {
                    eprintln!("Failed to read file {}: {:?}", file_path.display(), e);
                    // 在这里直接定义变量并推入
                    let error_package = DataStructPackage {
                        data_status: format!("Failed to read file: {:?}", e),
                        data_path: file_path,
                        data_struct: DataStruct { values: Vec::new() },
                    };
                    read_vec.data_structs.push(error_package);
                }
            }
        }
    }

    Ok(read_vec)
}




// 该模块的测试代码,Rust 的测试模块通常放在同一个文件中，使用 #[cfg(test)] 注解来标识测试模块。测试函数使用 #[test] 注解，并且可以使用 assert!、assert_eq! 等宏来验证代码的正确性。
//使用方法：在项目根目录下运行 cargo test 来执行测试。测试函数会自动被识别并执行，测试结果会显示在终端中。
#[cfg(test)]
mod tests {
    use super::*; // 引入父作用域中的函数和枚举
    use tempfile::NamedTempFile; // 需要在 Cargo.toml 中添加 tempfile 依赖
    use std::io::Write; // 用于写入临时文件

    // 测试 1: 验证有效扩展名的识别
    #[test]
    fn test_valid_formats_parsing() {
        assert!(ValidFormats::from_str("txt").is_ok());
        assert!(ValidFormats::from_str("JSON").is_ok()); // 验证大小写不敏感
        assert!(ValidFormats::from_str("exe").is_err());
    }

    // 测试 2: 验证文件校验逻辑（使用临时文件）
    #[test]
    fn test_check_file_valid() {
        // 创建一个临时的 .txt 文件
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("txt");
        
        // 由于 NamedTempFile 默认没后缀，我们手动创建一个带后缀的路径
        let path_str = path.to_str().unwrap();
        std::fs::File::create(path_str).unwrap();

        let result = check_file(path_str);
        assert!(result.is_ok());

        //创建一个临时的 .exe 文件
        let temp_file_exe = NamedTempFile::new().unwrap();
        let path_exe = temp_file_exe.path().with_extension("exe");
        let path_exe_str = path_exe.to_str().unwrap();
        std::fs::File::create(path_exe_str).unwrap();
        
        let result_exe = check_file(path_exe_str);  
        assert!(matches!(result_exe, Err(FileReadError::UnsupportedFormat)));

        // 清理测试产生的文件
        let _ = std::fs::remove_file(path_str);
        let _ = std::fs::remove_file(path_exe_str);

    }

    // 测试 3: 验证文件不存在的情况
    #[test]
    fn test_check_file_not_found() {
        let result = check_file("non_existent_file.xyz");
        //io::Error没有实现 PartialEq，所以不能直接比较错误类型，只能使用 matches! 来检查错误类型
        match result {
            Err(FileReadError::NotFound) => (),
            _ => panic!("应该返回 NotFound 错误"),
        }
    }

    // 测试 4: 验证无后缀文件
    #[test]
    fn test_check_file_no_extension() {
        let temp_file = NamedTempFile::new().unwrap();
        let path_str = temp_file.path().to_str().unwrap();
        
        let result = check_file(path_str);
        // 注意：NamedTempFile 生成的文件通常没有扩展名
        assert!(matches!(result, Err(FileReadError::NoExtension)));
    }


    //测试 5: 验证成功读取文件内容
    #[test]
    fn test_read_file_to_data_struct_success() {
        // 创建临时文件并写入测试数据
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "line1\nline2\nline3";
        writeln!(temp_file, "{}", content).unwrap();

        // 由于 check_file 依赖后缀，我们需要给临时文件路径加个后缀
        let path = temp_file.path().with_extension("txt");
        std::fs::copy(temp_file.path(), &path).unwrap();

        let path_str = path.to_str().unwrap();
        let result = read_file_to_data_struct(path_str);

        // 验证读取结果
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.data_struct.values.len(), 3);
        assert_eq!(data.data_struct.values[0], "line1");
        assert_eq!(data.data_struct.values[2], "line3");

        // 清理
        let _ = std::fs::remove_file(path);
    }

    // 测试 6: 验证当文件不存在时，应返回 NotFound (由 check_file 触发)
    #[test]
    fn test_read_file_not_found() {
        let result = read_file_to_data_struct("this_file_does_not_exist.txt");
        assert!(matches!(result, Err(FileReadError::NotFound)));
    }

    // 测试 7: 验证空文件读取
    #[test]
    fn test_read_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("csv");
        std::fs::File::create(&path).unwrap();

        let path_str = path.to_str().unwrap();
        let result = read_file_to_data_struct(path_str);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().data_struct.values.len(), 0);

        let _ = std::fs::remove_file(path);
    }

}
