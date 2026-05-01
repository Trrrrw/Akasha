use axum::Json;
use serde::Serialize;
use sysinfo::{Pid, System};
use utoipa::ToSchema;

use axum_mcp::mcp;

#[mcp]
#[utoipa::path(
    get,
    path = "/status",
    tag = "System",
    summary = "获取服务器状态",
    description = "返回服务器 CPU、内存、运行时间，以及当前后端进程的基础资源占用。",
    responses(
        (status = 200, body = SystemStatusResponse)
    )
)]
pub async fn status() -> Json<SystemStatusResponse> {
    let mut system = System::new_all();
    system.refresh_all();

    let process = sysinfo::get_current_pid().ok().and_then(|pid| {
        system
            .process(Pid::from(pid.as_u32() as usize))
            .map(ProcessStatus::from)
    });

    Json(SystemStatusResponse {
        cpu: CpuStatus {
            cores: system.cpus().len(),
            global_usage_percent: system.global_cpu_usage(),
        },
        memory: MemoryStatus {
            total_bytes: system.total_memory(),
            used_bytes: system.used_memory(),
            available_bytes: system.available_memory(),
            total_swap_bytes: system.total_swap(),
            used_swap_bytes: system.used_swap(),
        },
        uptime_seconds: System::uptime(),
        load_average: LoadAverageStatus::from(System::load_average()),
        process,
    })
}

#[derive(Serialize, ToSchema)]
#[schema(description = "服务器当前状态响应。")]
pub struct SystemStatusResponse {
    /// CPU 状态。
    cpu: CpuStatus,
    /// 内存状态，单位为字节。
    memory: MemoryStatus,
    /// 系统启动至今的秒数。
    uptime_seconds: u64,
    /// 系统平均负载。
    load_average: LoadAverageStatus,
    /// 当前后端进程状态。
    process: Option<ProcessStatus>,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "CPU 状态。")]
pub struct CpuStatus {
    /// CPU 逻辑核心数。
    cores: usize,
    /// 全局 CPU 使用率，百分比。
    global_usage_percent: f32,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "内存状态，单位为字节。")]
pub struct MemoryStatus {
    /// 总内存。
    total_bytes: u64,
    /// 已使用内存。
    used_bytes: u64,
    /// 可用内存。
    available_bytes: u64,
    /// 总 swap。
    total_swap_bytes: u64,
    /// 已使用 swap。
    used_swap_bytes: u64,
}

#[derive(Serialize, ToSchema)]
#[schema(description = "系统平均负载。")]
pub struct LoadAverageStatus {
    /// 1 分钟平均负载。
    one: f64,
    /// 5 分钟平均负载。
    five: f64,
    /// 15 分钟平均负载。
    fifteen: f64,
}

impl From<sysinfo::LoadAvg> for LoadAverageStatus {
    fn from(value: sysinfo::LoadAvg) -> Self {
        Self {
            one: value.one,
            five: value.five,
            fifteen: value.fifteen,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[schema(description = "当前后端进程状态。")]
pub struct ProcessStatus {
    /// 进程 ID。
    pid: u32,
    /// 进程名。
    name: String,
    /// 进程 CPU 使用率，百分比。
    cpu_usage_percent: f32,
    /// 进程内存占用，单位为字节。
    memory_bytes: u64,
    /// 进程虚拟内存占用，单位为字节。
    virtual_memory_bytes: u64,
    /// 进程启动至今的秒数。
    run_time_seconds: u64,
}

impl From<&sysinfo::Process> for ProcessStatus {
    fn from(value: &sysinfo::Process) -> Self {
        Self {
            pid: value.pid().as_u32(),
            name: value.name().to_string_lossy().into_owned(),
            cpu_usage_percent: value.cpu_usage(),
            memory_bytes: value.memory(),
            virtual_memory_bytes: value.virtual_memory(),
            run_time_seconds: value.run_time(),
        }
    }
}
