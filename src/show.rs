/*用于可视化展示匹配结果*/
/*大致思路：
1. 分级展示：首先展示当前匹配的文件列表，选择一个文件之后展示文件内部的匹配结果
2. 翻页：文件列表和内部匹配结果都可以翻页，每页20行
3. 文件列表包括：序号、文件路径，高亮展示当前选定的文件
4. 匹配结果包括：文件路径（标题）、匹配行（高亮表示匹配部分）、行号

基于ratatui和crossterm实现
*/

use crate::data_struct::{DataStruct, FileContentPage, FilePageContainer, SearchHit};
use crate::config::get_config;
use anyhow::{Ok,Context};
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


enum AppView{
    FileList,
    SearchDetail,
}
    
struct AppState<'a> {
    view: AppView,
    //我选择了让AppState仅持有FilePageContainer的引用
    raw_data: &'a FilePageContainer<'a>,
    list_state: ListState,
    //注意，Rust禁止自己持有又指向自身一部分的指针，因此这里要不然是AppState直接持有FileContentPage，同时持有FilePageContainer，要不然就是AppState仅持有FilePageContainer的引用，同时持有指向FilePageContainer的另一个引用
    current_detail:Option<&'a FileContentPage<'a>>,
    current_file_path:Option<&'a Path> ,
}

impl<'a> AppState<'a>{
    fn new(data: &'a FilePageContainer<'a>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            view: AppView::FileList,
            raw_data: data,
            list_state,
            current_detail:None,
            current_file_path:None
        }
    }
}


pub struct Visualizer;

impl Visualizer {
    /// 外部调用接口：传入数据并接管终端进行展示
    pub fn show<'a>(data: &'a FilePageContainer<'a>) -> anyhow::Result<()> {
        let mut app = AppState::new(data);
        
        // 渲染循环
        let mut terminal = setup_terminal()?;
        let res = Self::run_app(&mut terminal, &mut app);
        restore_terminal(terminal)?;
        
        res
    }

    pub fn process_key_action(app: &mut AppState) -> anyhow::Result<()> {
        if let Event::Key(key) = event::read()? {
            match app.view {
                AppView::FileList => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => {Self::move_next(app);Ok(())},
                    KeyCode::Up => {Self::move_prev(app);Ok(())},
                    KeyCode::Enter => {
                        if get_config().on_test{
                            // 测试用数据（注意这里的Box::leak是强行构造持久变量的方法）
                            let static_ref = Self::mock_fetch_detail();
                            app.current_detail = Some(static_ref); 
                        }
                        else{
                            //获取当前的Path-key
                            if let Some(index) = app.list_state.selected() {
                                if let Some(selected_file) = app.raw_data.file_metadata_vec.get(index) {
                                    let file_path = &selected_file.file_path;
                                    app.current_detail = app.raw_data.get_file_content_page(file_path); 
                                    app.current_file_path = Some(file_path);
                                }
                            }
                        }
                        app.view = AppView::SearchDetail;

                        Ok(())

                    }
                    _ => Ok(())
                },
                AppView::SearchDetail => match key.code {
                    KeyCode::Esc | KeyCode::Backspace => {app.view = AppView::FileList; Ok(())},
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down => {Self::move_next(app);Ok(())},
                    KeyCode::Up => {Self::move_prev(app);Ok(())},
                    _ => Ok(())
                },
            }
        }
        else{
            return Ok(());
        }
    }

    fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> anyhow::Result<()> {
        loop {
            //anyhow包装的错误有硬性要求，直接?不满足要求，这里必须经过转换
            terminal.draw(|f| ui(f, app)).map_err(|e| anyhow::anyhow!("Terminal error: {:?}", e))?;
            //这里需要将app的可变借用独立成一个函数，此时检查器才不会报借用冲突
            Self::process_key_action(app);
        }
    }

    //ListState是专用于存储翻页等滚动的索引的
    fn move_next(app: &mut AppState) {
        let i = match app.list_state.selected() {
            Some(i) => if i >= app.raw_data.page_metadata.as_ref().unwrap().total_pages - 1 { 0 } else { i + 1 },
            None => 0,
        };
        app.list_state.select(Some(i));
    }

    fn move_prev(app: &mut AppState) {
        let i = match app.list_state.selected() {
            Some(i) => if i == 0 { app.raw_data.page_metadata.as_ref().unwrap().total_pages - 1 } else { i - 1 },
            None => 0,
        };
        app.list_state.select(Some(i));
    }

    // 模拟根据 ID 获取文件内容的分页数据
    fn mock_fetch_detail<'a>() -> &'static FileContentPage<'a> {
        
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

        let test_page = FileContentPage {
            query_result:query_result_vec,
        };


        let static_ref: &'static FileContentPage = Box::leak(Box::new(test_page));

        static_ref

    }
}

// --- UI 渲染函数 ---

fn ui(f: &mut Frame, app: & mut AppState) {
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
        AppView::FileList => render_file_list(f, app, chunks[0]),
        AppView::SearchDetail => render_search_detail(f, app, chunks[0]),
    }

    // 底部帮助栏
    let help_text = match app.view {
        AppView::FileList => "↑/↓: 移动 | Enter: 查看详情 | q: 退出",
        AppView::SearchDetail => "Esc: 返回列表 | q: 退出",
    };
    f.render_widget(Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray)), chunks[1]);
}

//渲染元数据列表的逻辑
fn render_file_list(f: &mut Frame, app: &mut AppState, area: Rect) {
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
            //动态显示页码(这里需要看一下翻页逻辑是否实现了)
            .title(format!(" Files (Page {}/{}) ", app.raw_data.page_metadata.as_ref().unwrap().current_page, &app.raw_data.page_metadata.as_ref().unwrap().total_pages)))
        .highlight_style(Style::default()
            .bg(Color::Blue)//选中项背景变蓝
            .add_modifier(Modifier::BOLD))//选中项加粗
        .highlight_symbol(">> ");//选中项左侧加上这个前缀
    
    //在这里，框架通过查看ListState获取被选中的行，并进行特殊渲染
    f.render_stateful_widget(list, area, &mut app.list_state);
}


//渲染检索结果的逻辑
fn render_search_detail(f: &mut Frame, app: &AppState, area: Rect) {
    if let Some(detail) = &app.current_detail {
        let mut text = Vec::new();
        
        for q_line in &detail.query_result {
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
            .block(Block::default().borders(Borders::ALL).title(format!(" File: {} ", app.current_file_path.unwrap().display())));
        f.render_widget(p, area);
    }
}

// --- 终端环境设置 ---

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