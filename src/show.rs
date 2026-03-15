/*用于可视化展示匹配结果*/
/*大致思路：
1. 分级展示：首先展示当前匹配的文件列表，选择一个文件之后展示文件内部的匹配结果
2. 翻页：文件列表和内部匹配结果都可以翻页，每页20行
3. 文件列表包括：序号、文件路径，高亮展示当前选定的文件
4. 匹配结果包括：文件路径（标题）、匹配行（高亮表示匹配部分）、行号

基于ratatui和crossterm实现
*/

use crate::data_struct::{FileContentData, FilePageContainer, SearchHit, FileContentPage};
use anyhow::Ok;
use ratatui::{
    backend::{Backend,CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
    
};
use crossterm::event::{self, Event, KeyCode};
use crossterm::ExecutableCommand;
use std::io;
use std::path::Path;
use std::time::Duration;

//加Copy可以避免可能的借用冲突
#[derive(Clone, Copy, PartialEq,Debug)]
enum AppView{
    FileList,
    SearchDetail,
}
    
struct AppState<'a> {
    view: AppView,
    //我选择了让AppState仅持有FilePageContainer的引用
    //这里要注意，这个地方就需要让raw_data可变，因为后面会有数据处理等过程，需要这个数据是可变的
    raw_data: &'a mut FilePageContainer<'a>,
    list_state: ListState,
    //注意，Rust禁止自己持有又指向自身一部分的指针，因此这里要不然是AppState直接持有FileContentData，同时持有FilePageContainer，要不然就是AppState仅持有FilePageContainer的引用，同时持有指向FilePageContainer的另一个引用
    //这种会变的字段不要存在大结构体里，随用随生成吧，要不然过不了Rust生命周期检查，这里要不然就是随用随生成，要不然就要copy，想从自身成员指向自身成员是Rust不怎么支持的
    // current_detail:Option<FileContentPage<'a>>,
    current_file_path:Option<&'a Path> ,
    pub should_quit:bool,
}

impl<'a> AppState<'a>{
    fn new(data: &'a mut FilePageContainer<'a>) -> Self {
        let mut list_state = ListState::default();
        //从1开始
        list_state.select(Some(0));
        Self {
            view: AppView::FileList,
            raw_data: data,
            list_state,
            // current_detail:None,
            current_file_path:None,
            should_quit:false,
        }
    }
}


pub struct Visualizer;

impl Visualizer {
    /// 外部调用接口：传入数据并接管终端进行展示
    pub fn show<'a>(data: &'a mut FilePageContainer<'a>) -> anyhow::Result<()> {
        let mut app = AppState::new(data);
        Self::init_page_metadata(& mut app);
        let mut terminal = setup_terminal()?;
        // 清空启动时缓冲的按键，避免被误当作第一次输入（如自动触发 Enter）
        drain_pending_events();
        let res = Self::run_app(&mut terminal, &mut app);
        restore_terminal(terminal)?;
        res
    }

    //将按键处理的状态机逻辑剥离出来（将event::read()这个阻塞式且依赖环境的I/O操作单独放置，将可测试的逻辑拆出来）以便于测试
    pub fn handle_key_event(app: &mut AppState, key: event::KeyEvent)-> anyhow::Result<()>{
        /*
        crossterm中，KeyEvent包括Press、Repeat、Release等，下面的逻辑代码没有按照这些做区分，如果不屏蔽，会对每一种key.code都执行move_next/move_prev，导致翻页总是一下两页
        在这里，屏蔽掉其他的逻辑，仅处理“按下”事件，即可实现按一下动一下的效果
        */
        if !key.is_press() {
            return Ok(());
        }
        match app.view {
            AppView::FileList => match key.code {
                KeyCode::Char('q') =>{app.should_quit=true;Ok(())},
                KeyCode::Down => {Self::move_next(app);  Ok(())},
                KeyCode::Up => {Self::move_prev(app);Ok(())},
                KeyCode::Enter => {
                        //获取当前的Path-key
                        if let Some(index) = app.list_state.selected() {
                            //这里要防止超过最小index,saturating_sub是保证不小于0的写法，另外，usize是不能小于0的
                            let selected_file = app.raw_data.file_metadata_vec.get(index).unwrap();
                            let file_path = &selected_file.file_path;
                            app.current_file_path = Some(file_path);
                        }
                    app.view = AppView::SearchDetail;
                    // 进入文件内容视图时必须初始化内容页分页元数据，否则 Down/Up 使用的是文件列表的 total_pages，导致翻页无反应
                    Self::init_page_metadata(app);
                    app.list_state.select(Some(0));

                    Ok(())

                }
                _ => Ok(())
            },
            AppView::SearchDetail => match key.code {
                KeyCode::Esc | KeyCode::Backspace => {app.view = AppView::FileList; Self::init_page_metadata(app); Ok(())},
                KeyCode::Char('q') => {app.should_quit=true;Ok(())},
                KeyCode::Down => {
                    Self::move_next(app);
                    Ok(())},
                KeyCode::Up => {Self::move_prev(app);  Ok(())},
                _ => Ok(())
            },
        }


    }

    //将不好测试的逻辑单独放在一个地方
    pub fn process_key_action(app: &mut AppState) -> anyhow::Result<()> {
        if let Event::Key(key) = event::read()? {
            Self::handle_key_event(app, key)?;
        }
        Ok(())
    }

    fn init_page_metadata(app:&mut AppState){

        //这里的意思是，通过模式匹配将里面的内容解耦出来，这样就可以分别改其中的一部分，而不是整个大容器传来传去
        let AppState { view, raw_data, current_file_path, .. } = app;

        match view{
            AppView::FileList=> {raw_data.new_metadata_page()},
            AppView::SearchDetail=>{

                raw_data.new_content_page(current_file_path.as_ref().unwrap())
            },
        };

    }

    fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> anyhow::Result<()> {
        loop {
            if app.should_quit{
                return Ok(())
            }
            {
                //令raw_data准备好展示的元数据(挪到各个动作执行完的地方)
                // Self::init_page_metadata(app);
            }

            {
                //anyhow包装的错误有硬性类型要求，直接?不满足要求，这里必须经过转换
                terminal.draw(|f| ui(f, app)).map_err(|e| anyhow::anyhow!("Terminal error: {:?}", e))?;
            }

            {
                //这里需要将app的可变借用独立成一个函数，此时检查器才不会报借用冲突
                if let Err(e) = Self::process_key_action(app) {
                    return Err(e);
                }
            }
        }
    }

    //ListState是专用于存储翻页等滚动的索引的。注意：不要用 select()，否则会把 offset 重置为 0 导致列表总是跳回第一行
    fn move_next(app: &mut AppState) {

        match app.view{
            AppView::FileList=>{
                let i = match app.list_state.selected() {
                    Some(i) => if i >= (app.raw_data.page_metadata.as_ref().unwrap().total_lines.saturating_sub(1)) { 0 } else { i+1 },
                    None => 0,
                };
                app.list_state.select(Some(i));
            },
            AppView::SearchDetail=>{
                let i = match app.list_state.selected() {
                    Some(i) => if i >= (app.raw_data.page_metadata.as_ref().unwrap().total_pages.saturating_sub(1)) { 0 } else { i+1 },
                    None => 0,
                };
                app.list_state.select(Some(i));
                let _ = app.raw_data.next_page();
            }
        }

    }

    fn move_prev(app: &mut AppState) {

        match app.view{
            AppView::FileList=>{
                let total = app.raw_data.page_metadata.as_ref().unwrap().total_lines;
                let i = match app.list_state.selected() {
                    Some(i) => if i == 0 { total.saturating_sub(1) } else { i.saturating_sub(1) },
                    None => 0,
                };
                app.list_state.select(Some(i));
            },
            AppView::SearchDetail=>{
                let total = app.raw_data.page_metadata.as_ref().unwrap().total_pages;
                let i = match app.list_state.selected() {
                    Some(i) => if i == 0 { total.saturating_sub(1) } else { i.saturating_sub(1) },
                    None => 0,
                };
                app.list_state.select(Some(i));
                let _ = app.raw_data.prev_page();
            }
        }

    }


}



// --- UI 渲染函数 ---

fn ui(f: &mut Frame, app: &mut AppState) {
    //将屏幕分成两个区域：chunks[0]主内容区和chunks[1]工具栏区
    let chunks = Layout::default()
        //垂直排列
        .direction(Direction::Vertical)
        //四周留白1个单位
        .margin(1)
        //上层占据所有空间，下层固定高度为1
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        //按照当前窗口大小切割
        .split(f.area());

    match app.view {
        AppView::FileList => {
            render_file_list(f, app, chunks[0])
        },
        AppView::SearchDetail => {
            if let Some(path) = app.current_file_path {
                // 现场获取切片，生命周期仅限于此闭包，完美避开报错
                let detail = app.raw_data.get_file_content_page(path); 
                // render_detail_view(f, &detail, f.size());
                render_search_detail(f, &detail,path, chunks[0])}
                
            }
            
    }

    // 底部帮助栏
    let help_text = match app.view {
        AppView::FileList => "↑/↓: 移动 | Enter: 查看详情 | q: 退出",
        AppView::SearchDetail => "Esc: 返回列表 | q: 退出",
    };
    f.render_widget(Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray)), chunks[1]);
}

//渲染元数据列表的逻辑
fn render_file_list(f: &mut Frame, app: & mut AppState, area: Rect) {


    //将数据转换为UI组件
    let items: Vec<ListItem> = app.raw_data.file_metadata_vec.iter()
        .enumerate()
        .map(|(i, meta)| {
            //在这里加上序号
            //注意，PathBuf不能直接打印，需要通过display()方法转换
            ListItem::new(format!("{}. {}", i + 1, meta.file_path.display()))
        })
        .collect();
    
    //定义样式与外观
    let list = List::new(items)
        .block(Block::default()
            //加边框
            .borders(Borders::ALL)
            //动态显示页码
            .title(format!(" Files (Page {}/{}) ", app.raw_data.page_metadata.as_ref().unwrap().current_page, &app.raw_data.page_metadata.as_ref().unwrap().total_pages)))
        .highlight_style(Style::default()
            .bg(Color::Blue)//选中项背景变蓝
            .add_modifier(Modifier::BOLD))//选中项加粗
        .highlight_symbol(">> ");//选中项左侧加上这个前缀
    
    //在这里，框架通过查看ListState获取被选中的行，并进行特殊渲染
    f.render_stateful_widget(list, area, &mut app.list_state);
}


//渲染检索结果的逻辑
//注意这里的实现，这里没有传入app，而是仅传入所需字段,并且现用现生成（为避免引用冲突问题）
fn render_search_detail(f: &mut Frame, current_detail: &Option<FileContentPage>,current_path:&Path, area: Rect) {
    if let Some(detail) = current_detail {
        let mut text = Vec::new();
        
        //切片直接迭代，非切片需要放引用（为了模式匹配）
        for q_line in detail.query_result_page {
            // 实现高亮：将一行拆分为 [前缀, 匹配项, 后缀]
            let content = &q_line.matched_line;
            let mut spans = Vec::new();
            
            //为行号添加前缀
            spans.push(Span::styled(
                format!("{:>4} | ", q_line.line_number), 
                Style::default().fg(Color::DarkGray)
            ));

            let mut current_pos = 0;
            
            //保证进行切片和重排时的顺序是正确的
            let mut positions = q_line.hit_position_vec.clone();
            positions.sort_by_key(|&(start, _)| start);


            for &(start, end) in &positions {
                // 检查是否有关键词之间的普通文本
                if start > current_pos {
                    spans.push(Span::raw(&content[current_pos..start]));
                }
                
                // 添加高亮文本
                spans.push(Span::styled(
                    &content[start..end],
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                ));
                
                current_pos = end;
            }
            //添加剩余的普通文本（如果有）
            if current_pos < content.len() {
                spans.push(Span::raw(&content[current_pos..]));
            }


            let final_line = Line::from(spans);
            text.push(final_line);

        }

        let p = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(format!(" File: {} ", current_path.display())));
        f.render_widget(p, area);
    }
}

// --- 终端环境设置 ---

/// 清空当前未处理的按键事件，避免启动时缓冲的 Enter 等被当作第一次输入
fn drain_pending_events() {
    while event::poll(Duration::ZERO).unwrap_or(false) {
        let _ = event::read();
    }
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    
    let mut stdout = io::stdout();
    
    // 不再直接使用 execute! 宏，改用 trait 方法，这样类型推导更稳定
    // 这里的execute!宏会让编译器推导不出来正确的Error类型，导致错误类型匹配不上
    stdout.execute(crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| anyhow::anyhow!("Failed to enter alternate screen: {}", e))?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?; 
    
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    crossterm::terminal::disable_raw_mode()?;

    // 不再直接使用 execute! 宏，改用execute强行执行trait 方法，这样类型推导更稳定
    terminal.backend_mut().execute(crossterm::terminal::LeaveAlternateScreen)
        //错误类型强制转换
        .map_err(|e| anyhow::anyhow!("Failed to leave screen: {}", e))?;
    
    terminal.show_cursor()?;
    Ok(())
}

// 模拟根据 ID 获取文件内容的分页数据
fn mock_fetch_detail<'a>() -> FileContentPage<'a> {
    
    let line_for_test_1:&'static str = "there is a test line";
    let line_for_test_2:&'static str = "A cat just in there";

    let mut query_result_vec = Vec::new();
    let sh_1 = SearchHit{
        hit_position_vec:vec![(0,5)],
        line_number:1,
        matched_line:line_for_test_1,
    };
    query_result_vec.push(sh_1);


    let sh_2 = SearchHit{
        hit_position_vec:vec![(15,20)],
        line_number:2,
        matched_line:line_for_test_2,
    };
    query_result_vec.push(sh_2);

    let test_data = FileContentData {
        query_result:query_result_vec,
    };


    let static_ref: &'static FileContentData = Box::leak(Box::new(test_data));

    let fcp = FileContentPage{
        query_result_page:&static_ref.query_result[0..static_ref.query_result.len()]
    };

    fcp
    

}


#[cfg(test)]
mod tests{
    use super::*;
    use std::{hash::Hash, path::PathBuf};
    use std::collections::HashMap;
    // 从 data_struct 引入以下结构体来构造测试数据
    use crate::data_struct::{FilePageContainer, PageMetaData, FileMetadata};
    use crate::config;

    

    fn raw_mock_fetch_detail<'a>() -> (FileContentData<'a>,FileContentData<'a>) {
        
        let line_for_test_1:&'static str = "there is a test line";
        let line_for_test_2:&'static str = "A cat just in there";

        let mut query_result_vec = Vec::new();

        let sh_1 = SearchHit{
            hit_position_vec:vec![(0,5)],
            line_number:1,
            matched_line:line_for_test_1,
        };
        query_result_vec.push(sh_1);


        let sh_2 = SearchHit{
            hit_position_vec:vec![(15,20)],
            line_number:2,
            matched_line:line_for_test_2,
        };
        query_result_vec.push(sh_2);

        let test_page = FileContentData {
            query_result:query_result_vec,
        };


        let line_for_test_3:&'static str = "is a test line there";
        let line_for_test_4:&'static str = "there A cat just in";

        let mut query_result_vec_2 = Vec::new();

        let sh_3 = SearchHit{
            hit_position_vec:vec![(16,21)],
            line_number:1,
            matched_line:line_for_test_3,
        };
        query_result_vec_2.push(sh_3);


        let sh_4 = SearchHit{
            hit_position_vec:vec![(0,5)],
            line_number:2,
            matched_line:line_for_test_4,
        };
        query_result_vec_2.push(sh_4);

        let test_page_2 = FileContentData {
            query_result:query_result_vec_2,
        };

        (test_page,test_page_2)

    }
    

    /// 辅助函数：构造一个用于测试的 FilePageContainer 假数据
    fn create_mock_container<'a>() -> FilePageContainer<'a> {
        let page_meta = PageMetaData {
            current_page: 1,
            total_pages: 1,
            page_size:20,
            total_lines:3,
        };
        let file_pathbuf_1:&'static PathBuf = Box::leak(Box::new(PathBuf::from("file1.txt")));
        let file_pathbuf_2:&'static PathBuf =Box::leak(Box::new(PathBuf::from("file2.txt")));

        let meta1 = FileMetadata { file_path:&file_pathbuf_1 };
        let meta2 = FileMetadata { file_path:&file_pathbuf_2 };

        let (fcps_1,fcps_2) = raw_mock_fetch_detail();

        //HashMap要这样创建，生命周期写在类型声明处，新建用default()
        let mut shm:HashMap<&'a Path, FileContentData<'a>> = HashMap::default();

        shm.insert(file_pathbuf_1.as_path(), fcps_1);
        shm.insert(file_pathbuf_2.as_path(), fcps_2);


        FilePageContainer {
            page_metadata: Some(page_meta),
            file_metadata_vec: vec![meta1, meta2],
            search_hit_map:shm,
        }
        
        // unimplemented!("请根据你的 data_struct 补全 mock 数据的构造") 
    }

    #[test]
    fn test_app_state_initialization() {
        // 1. 准备测试数据
        let mut container = create_mock_container();
        
        // 2. 初始化 AppState
        let app = AppState::new(& mut container);

        // 3. 断言初始状态是否符合预期
        assert!(matches!(app.view, AppView::FileList), "初始视图应当是 FileList");
        assert_eq!(app.list_state.selected(), Some(0), "列表应当默认选中第 1 项");
        assert!(app.current_file_path.is_none(), "初始文件路径应当为空");

    }

    #[test]
    fn test_move_next() {
        let mut container = create_mock_container();
        // 假设总页数 (total_pages) 为 3，当前列表项从 0 开始
        let mut app = AppState::new(&mut container);

        // 初始选中 0
        assert_eq!(app.list_state.selected(), Some(0));

        // 移动到下一项
        Visualizer::move_next(&mut app);
        assert_eq!(app.list_state.selected(), Some(1));

        // 再次移动
        Visualizer::move_next(&mut app);
        assert_eq!(app.list_state.selected(), Some(2)); // 这里到达边界 (total_pages - 1)

        // 测试边界回绕 (Wrap around)
        Visualizer::move_next(&mut app);
        assert_eq!(app.list_state.selected(), Some(0), "超出边界后应当回到 0");
    }

    #[test]
    fn test_move_prev() {
        let mut container = create_mock_container();
        // 假设总页数 (total_pages) 为 3
        let mut app = AppState::new(&mut container);

        // 初始选中 0
        assert_eq!(app.list_state.selected(), Some(0));

        // 在 0 的位置向上移动，应当触发回绕，移动到最后一条 (total_pages - 1)
        Visualizer::move_prev(&mut app);
        assert_eq!(app.list_state.selected(), Some(2), "在顶部向上移动应当回绕到最后一条");

        // 正常向上移动
        Visualizer::move_prev(&mut app);
        assert_eq!(app.list_state.selected(), Some(1));
    }

    #[test]
    fn test_key_down_in_file_list() {
        let mut container = create_mock_container();
        let mut app = AppState::new(&mut container);
        config::init(); 

        // 模拟按下 Down 键
        let key_event = event::KeyEvent::new(KeyCode::Down, event::KeyModifiers::NONE);
        Visualizer::handle_key_event(&mut app, key_event).unwrap();
        
        assert_eq!(app.list_state.selected(), Some(1));

        let key_event = event::KeyEvent::new(KeyCode::Enter, event::KeyModifiers::NONE);
        Visualizer::handle_key_event(&mut app, key_event).unwrap();

        assert!(!app.current_file_path.is_none());
        // 进入详情后 list_state 表示内容页索引（0-based），应为第一页 0
        assert_eq!(app.list_state.selected(), Some(0));
        assert_eq!(app.view, AppView::SearchDetail);
    }

}