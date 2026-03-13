/*
基于file_read获取的文件数据，本模块实现对数据的解封装和查询功能，提供一个接口，允许查询数据结构中的内容。
当前仅实现声明支持的文件解读
*/

use crate::{data_struct::{DataStruct, DataStructPackage, FileContentPage, FileMetadata, FilePageContainer, PageMetaData, PageMetaDataContainer, SearchHit}};
use std::collections::HashMap;
use crate::config::{get_config};
use std::path::{Path};

enum QueryMode {
    Keyword(String),
    Regex(String),
}

#[derive(Debug)]
pub enum QueryError{
    EmptyQuery,
}

//加这两个是为了便于打印和测试断言
#[derive(Debug,PartialEq)]
pub struct QueryResultRef<'a>{
    //让结构体持有Vec，但Vec中的元素是对DataStruct中字符串的引用，这样就避免了数据复制，提高了效率
    pub matched_lines: Vec<&'a String>,
    pub data_path: &'a str,
}

fn keyword_match<'a>(data_struct: &'a DataStruct, keyword: &str) -> Vec<SearchHit<'a>> {
    data_struct.values.iter()
    .enumerate()
    .filter_map(|(idx,line)| {
        let matches:Vec<(usize, usize)> = line
            .match_indices(keyword)
            .map(|(start,matched)| (start,start+matched.len()))
            .collect();

        if matches.is_empty() {
            None
        }else{
            Some(SearchHit{
                    matched_lines: line,
                    line_number: idx + 1,
                    hit_position_vec: matches,
            })
        }
        
        }).collect()
}
    
//packages用切片，更通用，能传入vec等
pub fn query_data_struct<'a>(data_struct_packages: &'a [&'a DataStructPackage], query: &str) -> Result<FilePageContainer<'a>,QueryError> {
    // 这里可以实现正则匹配、关键字匹配等的逻辑
    // 目前仅简单实现关键字匹配
    if query.is_empty() {
        return Err(QueryError::EmptyQuery)
    }

    let mut metadata_vec: Vec<FileMetadata>=Vec::new();
    let mut search_hit_map:HashMap<&'a Path, FileContentPage<'a>>=HashMap::new();
    // let mut pmp:HashMap<usize, PageMetaData>=HashMap::new();

    //遍历所有的file，获取结果
    //这里记录meta_data、记录数据map
    data_struct_packages.iter()
    .enumerate()
    //map、filter_map必须有collect()才执行，for_each是直接执行
    .for_each(|(idx,data_struct_package)|{
            
            //添加文件metadata
            metadata_vec.push(FileMetadata{file_path:&data_struct_package.data_path});
            
            let lines_len;
            //获取检索结果
            let matched_lines = keyword_match(&data_struct_package.data_struct, query);

            if matched_lines.is_empty(){
                lines_len = 0;
            }
            else{
                lines_len = matched_lines.len();
            }

            let  fcp = FileContentPage{
                query_result:matched_lines
            };
            search_hit_map.insert(&data_struct_package.data_path, fcp);

        }
    );
    


    let fpc = FilePageContainer{
        page_metadata : None,
        file_metadata_vec:metadata_vec,
        search_hit_map:search_hit_map,
    };
    

    Ok(fpc)

}

//cargo test content_check 可以仅测试这个模块
//cfg(test) 是 Rust 中的一个条件编译属性，用于标记仅在测试环境下编译和运行。
#[cfg(test)]
mod tests{
    use crate::data_struct::DataStatus;

    use super::*; // 引入父作用域中的函数和结构体

    #[test]
    fn test_query_data_struct() {
        let data_struct = DataStruct {
            values: vec![
                "This is a test line.".to_string(),
                "Another line with keyword.".to_string(),
                "No match here.".to_string(),
            ],
        };

        let path_obj = std::path::Path::new("D:/test");
        
        let data_struct_package = DataStructPackage{
            data_path:path_obj.to_path_buf(),
            data_struct:data_struct,
            data_status:DataStatus::SUCCESS,
        };

        let mut dsp_vec = Vec::new();
        dsp_vec.push(&data_struct_package);

        //在不能继续抛出错误时，不能用?
        let result = query_data_struct(&dsp_vec, "keyword").unwrap();

        //unwrap()也能解option包
        assert_eq!(result.search_hit_map.get(path_obj).unwrap().query_result.len(), 1);
        assert_eq!(result.search_hit_map.get(path_obj).unwrap().query_result[0].matched_lines, &"Another line with keyword.".to_string());

        // 测试空查询
        let result = query_data_struct(&dsp_vec, "");
        //matches是地道写法，判断是否为Err分支和对应的特定错误    
        assert!(matches!(result, Err(QueryError::EmptyQuery)));
        

        // 测试没有匹配项
        let result = query_data_struct(&dsp_vec, "Python").unwrap();
        assert!(result.search_hit_map.get(path_obj).unwrap().query_result.is_empty());


    }



}

