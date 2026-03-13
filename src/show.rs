/*用于可视化展示匹配结果*/
/*大致思路：
1. 分级展示：首先展示当前匹配的文件列表，选择一个文件之后展示文件内部的匹配结果
2. 翻页：文件列表和内部匹配结果都可以翻页，每页20行
3. 文件列表包括：序号、文件路径，高亮展示当前选定的文件
4. 匹配结果包括：文件路径（标题）、匹配行（高亮表示匹配部分）、行号

基于ratatui和crossterm实现
*/

use crate::content_check::QueryResult;
use crate::data_struct::DataStruct;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use crossterm::event::{self, Event, KeyCode};
use std::io;


enum AppView{
    FileList,
    MatchDetail,
}



fn display_result(query_result:&QueryResult){
    println!("File: {}", query_result.data_path);
    for line in &query_result.matched_lines {
        println!("{}", line);
    }
}
