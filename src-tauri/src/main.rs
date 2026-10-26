// 阻止 Windows 发布版弹出额外的控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 命令行诊断模式：不启动 GUI，直接输出真实设备与切换结果
    if args.first().map(|arg| arg == "--diagnose").unwrap_or(false) {
        std::process::exit(audio_switch_lib::diagnose::run(&args));
    }

    audio_switch_lib::run()
}
