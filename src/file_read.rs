/*该模块实现文件读取功能,以及从目录中批量读取可用文件*/

use crate::data_struct::{DataStruct, DataStructPackage, DataStructVec,DataStatus};
use std::fs::{self,File};
use std::path::{Path,PathBuf};
use std::str::FromStr;
use std::io::{BufRead,BufReader, ErrorKind};
use thiserror::Error;
use std::fmt;
use std::io;


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
    NotAFile,
    IOError(std::io::Error),
}

//用于print以及转换为字符串
impl fmt::Display for FileReadError {
    fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result{
        match self {

            Self::NotFound=>write!(f,"NotFound"),
            Self::FileTooLarge=>write!(f,"FileTooLarge"),
            Self::UnsupportedFormat=>write!(f,"UnsupportedFormat"),
            Self::NoExtension=>write!(f,"NoExtension"),
            //当枚举携带数据时，需要进行匹配和解构
            Self::ReadError(msg)=>write!(f,"ReadError:{}",msg),
            Self::NotAFile=>write!(f,"NotAFile"),
            Self::IOError(err)=>write!(f,"IOError:{}",err),
        }
    }
}

#[derive(Debug)]
pub enum PathReadError {
    NotFound,
    NotADirectory,
    PermissionDenied,
    IOError(std::io::Error),
}


//展示规则
impl fmt::Display for PathReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "指定的路径未找到"),
            Self::PermissionDenied => write!(f, "没有权限访问该路径"),
            Self::NotADirectory => write!(f, "该路径不是一个目录"),
            Self::IOError(e) => write!(f, "IO 系统错误: {}", e),
        }
    }
}

//标准 Error 特征
impl std::error::Error for PathReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::IOError(e) = self {
            Some(e)
        } else {
            None
        }
    }
}

//从io::Error的自动转换
impl From<io::Error> for PathReadError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => PathReadError::NotFound,
            io::ErrorKind::PermissionDenied => PathReadError::PermissionDenied,
            io::ErrorKind::NotADirectory => PathReadError::NotADirectory,
            _ => PathReadError::IOError(error),
        }
    }
}


//检查文件是否合法,输入&PathBuf，这里用Path是提供泛型能力，&Path可以接受&PathBuf、&Path以及&str等
fn check_metadata(file: &File) -> Result<(), FileReadError> {

    //此处直接获取metadata同时进行检查
    let metadata = file.metadata().map_err(FileReadError::IOError)?;

    if !metadata.is_file() {
        return Err(FileReadError::NotAFile); // 建议增加这个错误变体
    }

    if metadata.len() > 10 * 1024 * 1024 {
        return Err(FileReadError::FileTooLarge);
    }

    Ok(())
}

pub fn read_file_to_data_struct(file_path: &Path) -> Result<DataStructPackage, FileReadError> {

    let ext = file_path.extension()
    //尝试转为字符串
    .and_then(|s| s.to_str())
    //如果不行，则传播错误
    .ok_or(FileReadError::NoExtension)?;

    //parse()方法是生成字符串，这里通过parse()方法来自动转换为ValidFormats（类型是手动标注的，所以可以知道转换目标）
    let _: ValidFormats = ext.parse().map_err(|_| FileReadError::UnsupportedFormat)?;

    //直接打开文件，避免重复路径查找，防止检查和打开间发生变化
    let file = File::open(file_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            FileReadError::NotFound
        }
        else {
            FileReadError::IOError(e)
        }
    })?;

    check_metadata(&file)?;

    let reader = BufReader::new(file);
    let mut values = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e|FileReadError::ReadError(e.to_string()))?;
        values.push(line);
    }

    let data_package = DataStructPackage {
        data_status: DataStatus::SUCCESS,
        data_path: file_path.to_path_buf(),
        data_struct:DataStruct{values},
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

    //  fs::read_dir 可能会因为权限等问题失败
    // 这里设计了Error的转换功能,通过from可以实现转换
    let entries = fs::read_dir(path).map_err(PathReadError::from)?;

    let mut data_structs_vec = Vec::new();

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
            match read_file_to_data_struct(&file_path) {
                Ok(data_package) => {
                    data_structs_vec.push(data_package);
                }
                Err(e) => {
                    eprintln!("Failed to read file {}: {:?}", file_path.display(), e);
                    // 在这里直接定义变量并推入
                    let error_package = DataStructPackage {
                        data_status: DataStatus::ERROR(e.to_string()),
                        data_path: file_path,
                        data_struct: DataStruct { values: Vec::new() },
                    };
                    data_structs_vec.push(error_package);
                }
            }
        }
    }

    let read_vec = DataStructVec {
        directory_path: path.to_string(),
        data_structs: data_structs_vec,
    };

    Ok(read_vec)
}




// 该模块的测试代码,Rust 的测试模块通常放在同一个文件中，使用 #[cfg(test)] 注解来标识测试模块。测试函数使用 #[test] 注解，并且可以使用 assert!、assert_eq! 等宏来验证代码的正确性。
//使用方法：在项目根目录下运行 cargo test 来执行测试。测试函数会自动被识别并执行，测试结果会显示在终端中。
#[cfg(test)]
mod tests {
    use super::*; // 引入父作用域中的函数和枚举
    use tempfile::{NamedTempFile,tempdir}; // 需要在 Cargo.toml 中添加 tempfile 依赖
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
        let path_obj = std::path::Path::new(path_str);

        let result = read_file_to_data_struct(path_obj);
        assert!(result.is_ok());

        //创建一个临时的 .exe 文件
        let temp_file_exe = NamedTempFile::new().unwrap();
        let path_exe = temp_file_exe.path().with_extension("exe");
        let path_exe_str = path_exe.to_str().unwrap();
        std::fs::File::create(path_exe_str).unwrap();
        let path_exe_obj = std::path::Path::new(path_exe_str);
        
        let result_exe = read_file_to_data_struct(path_exe_obj);  
        assert!(matches!(result_exe, Err(FileReadError::UnsupportedFormat)));

        // 清理测试产生的文件
        let _ = std::fs::remove_file(path_str);
        let _ = std::fs::remove_file(path_exe_str);

    }

    // 测试 3: 验证文件不存在的情况
    #[test]
    fn test_check_file_not_found() {

        let path_str = "d:/tetete.txt";
        let path_obj = std::path::Path::new(path_str);

        let result = read_file_to_data_struct(path_obj);
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
        let path_obj = std::path::Path::new(path_str);
        
        let result = read_file_to_data_struct(path_obj);
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
        let path_obj = std::path::Path::new(path_str);
        let result = read_file_to_data_struct(path_obj);

        // 验证读取结果
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.data_struct.values.len(), 3);
        assert_eq!(data.data_struct.values[0], "line1");
        assert_eq!(data.data_struct.values[2], "line3");
        assert!(matches!(data.data_status,DataStatus::SUCCESS));
        assert_eq!(data.data_path.to_str().unwrap(),path_str);

        // 清理
        let _ = std::fs::remove_file(path);
    }

    // 测试 6: 验证当文件不存在时，应返回 NotFound (由 check_file 触发)
    #[test]
    fn test_read_file_not_found() {
        let file_path = "this_file_does_not_exist.txt";
        let file_path_obj = std::path::Path::new(file_path);
        let result = read_file_to_data_struct(file_path_obj);
        assert!(matches!(result, Err(FileReadError::NotFound)));
    }

    // 测试 7: 验证空文件读取
    #[test]
    fn test_read_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().with_extension("csv");
        std::fs::File::create(&path).unwrap();

        let path_str = path.to_str().unwrap();
        let file_path_obj = std::path::Path::new(path_str);
        let result = read_file_to_data_struct(file_path_obj);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().data_struct.values.len(), 0);

        let _ = std::fs::remove_file(path);
    }

    //测试8：测试路径读取
    #[test]
    fn test_read_files_in_path_not_found() {
        let path = "D:/path/that/absolutely/does/not/exist_12345";
        let result = read_files_in_path(path);
        
        // 验证确实返回了错误
        assert!(result.is_err(), "Expected an error for non-existent path");
    }

    /// 测试9: 读取一个空目录
    /// 期望行为: 返回 Ok(DataStructVec)，且 data_structs 为空
    #[test]
    fn test_read_files_in_path_empty_dir() {
        // 创建临时目录
        let dir = tempdir().expect("Failed to create temp dir");
        let dir_path = dir.path().to_str().unwrap();

        let result = read_files_in_path(dir_path);
        
        assert!(result.is_ok());
        let read_vec = result.unwrap();
        assert_eq!(read_vec.directory_path, dir_path);
        assert!(read_vec.data_structs.is_empty(), "Data structs should be empty for an empty dir");
    }

    /// 测试10: 目录中包含有效文件、无效文件以及子目录
    /// 期望行为: 
    /// 1. 忽略子目录 (因为代码中有 file_path.is_file() 检查)
    /// 2. 成功解析有效文件
    /// 3. 无效文件被解析为带有 DataStatus::ERROR 的包
    #[test]
    fn test_read_files_in_path_mixed_contents() {
        let dir = tempdir().expect("Failed to create temp dir");
        let dir_path = dir.path();

        // 1. 创建一个正常文件 (假设你的 read_file_to_data_struct 能成功解析它)
        let valid_file_path = dir_path.join("valid.txt");
        let mut valid_file = File::create(&valid_file_path).unwrap();
        writeln!(valid_file, "valid content").unwrap(); // 请根据实际情况修改写入内容

        // 2. 创建一个错误文件 (假设它会导致 read_file_to_data_struct 返回 Err)
        let invalid_file_path = dir_path.join("invalid.exe");
        let mut invalid_file = File::create(&invalid_file_path).unwrap();
        writeln!(invalid_file, "invalid content").unwrap(); // 请根据实际情况修改写入内容

        // 3. 创建一个子目录 (应该被跳过)
        let sub_dir_path = dir_path.join("sub_dir");
        std::fs::create_dir(&sub_dir_path).unwrap();

        // 执行测试
        let result = read_files_in_path(dir_path.to_str().unwrap());
        
        assert!(result.is_ok());
        let read_vec = result.unwrap();

        // 验证返回结果
        assert_eq!(read_vec.directory_path, dir_path.to_str().unwrap());
        // 我们只放了 2 个文件（1 个子目录被忽略），所以应该刚好有 2 个结果
        assert_eq!(read_vec.data_structs.len(), 2);

        // 统计成功和失败的数量
        let mut error_count = 0;
        let mut success_count = 0;

        for package in read_vec.data_structs {
            match package.data_status {
                DataStatus::ERROR(_) => {
                    error_count += 1;
                    // 验证错误包的路径是那个 invalid 文件
                    // 注意：因为读取顺序是不确定的，最好通过判断文件名来确认
                    assert!(package.data_path.to_str().unwrap().contains("invalid.exe"));
                }
                _ => { // 假设你成功的状态是 DataStatus::OK 或是其他的
                    success_count += 1;
                    assert!(package.data_path.to_str().unwrap().contains("valid.txt"));
                }
            }
        }

        assert_eq!(success_count, 1, "Should have 1 successfully parsed file");
        assert_eq!(error_count, 1, "Should have 1 failed parsing parsed file");
    }


}
