/*
此处定义各种数据结构

需求：这个程序要实现以下几个功能：
1、读取合法文件，并将其内容存储在一个数据结构中。
    1、在windows下，合法文件的定义是：以.txt/.csv/.log/.dat/.JSON等结尾的文本文件，且文件大小不超过10MB。（后续会做扩展）
2、提供一个接口，允许查询数据结构中的内容。
3、基于以上接口，实现对数据结构中的内容进行查找，支持正则匹配、关键字匹配
4、该工具用于在终端中，对数据结构中的内容进行展示，支持分页显示和搜索功能。


功能设计：
程序大致可以分为四部分：
   - 文件读取模块：负责读取合法文件，并将其内容存储在数据结构中。
   - 数据查询模块：提供接口，允许查询数据结构中的内容。
   - 数据展示模块：在终端中展示数据结构中的内容，支持分页显示和搜索功能。
   - 主程序模块：负责协调各个模块的工作，处理用户输入和输出。


数据结构和接口设计
1. 文件读取模块的数据结构：
    文件读取模块需要一个数据结构来存储读取的文件内容。可以定义一个结构体 `DataStruct`，包含以下字段：
    - `id: u32`：数据结构的唯一标识符。 
    - `name: String`：数据结构的名称。
    - `values: Vec<String>`：存储文件内容的向量，每个元素代表文件中的一行数据。
2. 文件读取模块的接口设计：
    文件读取模块需要提供一个接口来读取合法文件并将其内容存储在 `DataStruct` 中。可以定义一个函数 `read_file_to_data_struct`，接受一个文件路径作为参数，返回一个 `DataStruct` 实例。
    ```rust
    fn read_file_to_data_struct(file_path: &str) -> Result<DataStruct, std::io::Error> {
        // 读取文件内容并存储在 DataStruct 中的逻辑
    }

3. 数据查询模块的数据结构：
    数据查询模块需要一个数据结构来存储查询结果。可以定义一个结构体 `QueryResult`，包含以下字段：
    - `matched_lines: Vec<String>`：存储匹配结果的向量，每个元素代表匹配到的一行数据。


4. 数据查询模块的接口设计：
    数据查询模块需要提供一个接口来查询数据结构中的内容。可以定义一个函数 `query_data_struct`，接受一个 `DataStruct` 实例和一个查询条件（如正则表达式或关键字），返回QueryResult类型的匹配结果。
    ```rust 

5. 数据展示模块的数据结构：
    数据展示模块需要一个数据结构来存储分页显示的内容。可以定义一个结构体 `Page`，包含以下字段：
    - `page_number: u32`：当前页码。
    - `page_size: u32`：每页显示的行数。
    - `total_pages: u32`：总页数。
    - `lines: Vec<String>`：存储当前页显示的行数据。

6. 数据展示模块的接口设计：
    数据展示模块需要提供一个接口来展示数据结构中的内容。可以定义一个函数 `display_page`，接受一个 `Page` 实例作为参数，在终端中展示当前页的内容。
    ```rust 

7. 主程序模块的接口设计：
    主程序模块需要协调各个模块的工作，处理用户输入和输出。可以定义一个函数 `main`，作为程序的入口点，负责调用文件读取、数据查询和数据展示模块的接口。
    ```rust
*/

use std::collections::HashMap;
use std::path::PathBuf;


pub struct DataStruct {
    pub values: Vec<String>,
}

pub struct DataStructPackage{
    pub data_status: String,
    pub data_path: PathBuf,
    pub data_struct: DataStruct,
}

pub struct DataStructVec{
    pub directory_path:String,
    pub data_structs: Vec<DataStructPackage>,
}

pub struct QueryLine{
    pub line_number:usize,
    pub content:String,
    pub match_range:Vec<(usize,usize)>,
}


pub struct SearchHit<'a> {
    pub matched_lines: &'a String,
    pub line_number: usize,
    pub hit_position_vec:Vec<(usize,usize)>
}

pub struct FileMetadata{
    pub file_path:String,
}

pub struct FileContentPage<'a> {
    pub query_result: Vec<SearchHit<'a>>,
}

pub struct PageMetaData{
    pub page_number: usize,
    pub page_size: usize,
    pub total_pages: usize,
    pub total_lines: usize,
}


pub struct PageMetaDataContainer{
    pub page_metadata_map:HashMap<usize, PageMetaData>,
}


impl PageMetaDataContainer {
    pub fn get_page_metadata(&self, page_index: usize) -> Option<&PageMetaData> {
        // 使用 self 访问成员变量
        self.page_metadata_map.get(&page_index)
    }

    pub fn set_page_medatada(&mut self,page_index:usize,pmd:PageMetaData)->(){
        self.page_metadata_map.insert(page_index, pmd);
    }

}


pub struct FilePageContainer<'a>{
    pub file_metadata_vec:Vec<FileMetadata>,
    pub page_metadata:PageMetaDataContainer,
    pub search_hit_map:HashMap<String, FileContentPage<'a>>,
}


/*

功能设计：
FilePageContainer存储文件列表、文件中匹配数据列表、展示页面的元数据
需要实现的功能：
1. 获取文件路径列表数据
2. 基于文件路径，从匹配数据中获取数据
3. 存储当前的展示页面数据
*/

//注意，带生命周期的是这样声明,前后都要有生命周期的声明符
impl <'a> FilePageContainer<'a>{

    pub fn get_file_metadata(&self)->&Vec<FileMetadata> {
        &self.file_metadata_vec
    }

    pub fn get_




}








