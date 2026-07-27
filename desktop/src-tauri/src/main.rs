// 仅 Release 发布版隐藏控制台，Debug调试保留终端
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    qingpan_lib::run();
}
