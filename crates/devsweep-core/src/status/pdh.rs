//! Read-only PDH probes for GPU engine utilization, ACPI thermal zones, CPU
//! frequency and system disk throughput.
//!
//! One query per sampler holds every counter and is collected once per sample.
//! A missing counter set, a missing instance, or a counter without valid data
//! becomes `unavailable` (or a `null` field) with a reason code. It never
//! becomes a zero value.

#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::BTreeMap;

#[cfg(windows)]
use windows_sys::Win32::System::Performance::{PDH_HCOUNTER, PDH_HQUERY};

use super::{AvailabilityV1, DiskActivityV1, GpuAdapterV1, GpuV1, ThermalV1, ThermalZoneV1};

/// English counter path for per-engine GPU utilization.
pub(crate) const GPU_ENGINE_COUNTER: &str = r"\GPU Engine(*)\Utilization Percentage";
/// English counter path for thermal-zone temperature in Kelvin.
pub(crate) const THERMAL_ZONE_COUNTER: &str = r"\Thermal Zone Information(*)\Temperature";
/// Nominal processor frequency in MHz.
pub(crate) const CPU_FREQUENCY_COUNTER: &str =
    r"\Processor Information(_Total)\Processor Frequency";
/// Processor performance relative to nominal, in percent (may exceed 100).
pub(crate) const CPU_PERFORMANCE_COUNTER: &str =
    r"\Processor Information(_Total)\% Processor Performance";
/// System-wide physical disk read throughput.
pub(crate) const DISK_READ_COUNTER: &str = r"\PhysicalDisk(_Total)\Disk Read Bytes/sec";
/// System-wide physical disk write throughput.
pub(crate) const DISK_WRITE_COUNTER: &str = r"\PhysicalDisk(_Total)\Disk Write Bytes/sec";

/// One formatted counter instance.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CounterSample {
    pub(crate) name: String,
    pub(crate) value: f64,
}

/// Instance values of one counter, or a stable reason code.
pub(crate) type CounterRead = Result<Vec<CounterSample>, &'static str>;

/// One single-instance counter value, or a stable reason code.
pub(crate) type ValueRead = Result<f64, &'static str>;

/// Effective frequency in MHz as Task Manager computes it: nominal frequency
/// times processor performance. Missing, non-finite or non-positive inputs
/// give `None`, never 0.
pub(crate) fn frequency_mhz(nominal: ValueRead, performance: ValueRead) -> Option<u32> {
    let (Ok(nominal), Ok(performance)) = (nominal, performance) else {
        return None;
    };
    if !nominal.is_finite() || !performance.is_finite() || nominal <= 0.0 || performance <= 0.0 {
        return None;
    }
    let mhz = (nominal * performance / 100.0).round();
    if mhz < 1.0 || mhz > f64::from(u32::MAX) {
        return None;
    }
    Some(mhz as u32)
}

/// System disk read/write throughput. Either counter failing makes the group
/// unavailable with that counter's reason; a negative or non-finite rate is
/// `no_valid_data`.
pub(crate) fn disk_activity_availability(
    read: ValueRead,
    write: ValueRead,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<DiskActivityV1> {
    let (read, write) = match (read, write) {
        (Ok(read), Ok(write)) => (read, write),
        (Err(reason), _) | (_, Err(reason)) => return AvailabilityV1::unavailable(reason),
    };
    match (bytes_per_second(read), bytes_per_second(write)) {
        (Some(read_bytes_per_second), Some(write_bytes_per_second)) => AvailabilityV1::available(
            sampled_at_unix_ms,
            0,
            DiskActivityV1 {
                read_bytes_per_second,
                write_bytes_per_second,
            },
        ),
        _ => AvailabilityV1::unavailable("no_valid_data"),
    }
}

fn bytes_per_second(value: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let rounded = value.round();
    if rounded >= u64::MAX as f64 {
        return Some(u64::MAX);
    }
    Some(rounded as u64)
}

/// Sum utilization per adapter and engine type, then keep the busiest engine
/// type per adapter. Instance names carry `luid_<..>_phys_<n>` and
/// `engtype_<type>`. `None` means no instance had a usable value.
pub(crate) fn gpu_from_samples(samples: &[CounterSample]) -> Option<GpuV1> {
    let mut sums: BTreeMap<(String, String), f64> = BTreeMap::new();
    for sample in samples {
        if !sample.value.is_finite() || sample.value < 0.0 {
            continue;
        }
        let Some((adapter, engine_type)) = gpu_instance_key(&sample.name) else {
            continue;
        };
        *sums.entry((adapter, engine_type)).or_insert(0.0) += sample.value;
    }
    if sums.is_empty() {
        return None;
    }
    let mut adapters: BTreeMap<String, f64> = BTreeMap::new();
    for ((adapter, _), sum) in sums {
        let busiest = adapters.entry(adapter).or_insert(0.0);
        if sum > *busiest {
            *busiest = sum;
        }
    }
    Some(GpuV1 {
        adapters: adapters
            .into_iter()
            .map(|(adapter_id, percent)| GpuAdapterV1 {
                adapter_id,
                utilization_basis_points: percent_to_basis_points(percent),
                name: None,
            })
            .collect(),
    })
}

/// Convert Kelvin to tenths of a degree Celsius. Zones that report 0 K or a
/// non-finite value are dropped as invalid. `None` means no valid zone.
pub(crate) fn thermal_from_samples(samples: &[CounterSample]) -> Option<ThermalV1> {
    let mut zones = samples
        .iter()
        .filter(|sample| sample.value.is_finite() && sample.value > 0.0)
        .map(|sample| ThermalZoneV1 {
            zone_id: sample.name.clone(),
            temperature_tenths_celsius: kelvin_to_tenths_celsius(sample.value),
        })
        .collect::<Vec<_>>();
    if zones.is_empty() {
        return None;
    }
    zones.sort_by(|left, right| left.zone_id.cmp(&right.zone_id));
    Some(ThermalV1 { zones })
}

pub(crate) fn gpu_availability(
    read: CounterRead,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<GpuV1> {
    match read {
        Ok(samples) => match gpu_from_samples(&samples) {
            Some(value) => AvailabilityV1::available(sampled_at_unix_ms, 0, value),
            None => AvailabilityV1::unavailable("no_instances"),
        },
        Err(reason) => AvailabilityV1::unavailable(reason),
    }
}

pub(crate) fn thermal_availability(
    read: CounterRead,
    sampled_at_unix_ms: u64,
) -> AvailabilityV1<ThermalV1> {
    match read {
        Ok(samples) => match thermal_from_samples(&samples) {
            Some(value) => AvailabilityV1::available(sampled_at_unix_ms, 0, value),
            None => AvailabilityV1::unavailable("no_instances"),
        },
        Err(reason) => AvailabilityV1::unavailable(reason),
    }
}

fn gpu_instance_key(name: &str) -> Option<(String, String)> {
    let phys = name.find("phys_")?;
    let adapter_end = phys + name[phys..].find("_eng_")?;
    let adapter_start = name.find("luid_").filter(|at| *at < phys).unwrap_or(phys);
    let engine_type = &name[name.find("engtype_")? + "engtype_".len()..];
    if engine_type.is_empty() {
        return None;
    }
    Some((
        name[adapter_start..adapter_end].to_string(),
        engine_type.to_string(),
    ))
}

fn percent_to_basis_points(percent: f64) -> u32 {
    let points = (percent * 100.0).round();
    if points >= 10_000.0 {
        10_000
    } else if points <= 0.0 {
        0
    } else {
        points as u32
    }
}

fn kelvin_to_tenths_celsius(kelvin: f64) -> i32 {
    let tenths = ((kelvin - 273.15) * 10.0).round();
    tenths.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// One PDH query with every Status counter. Dropping it closes the query.
#[derive(Debug)]
pub(crate) struct PdhProbe {
    #[cfg(windows)]
    query: PDH_HQUERY,
    #[cfg(windows)]
    gpu: Result<PDH_HCOUNTER, &'static str>,
    #[cfg(windows)]
    thermal: Result<PDH_HCOUNTER, &'static str>,
    #[cfg(windows)]
    cpu_frequency: Result<PDH_HCOUNTER, &'static str>,
    #[cfg(windows)]
    cpu_performance: Result<PDH_HCOUNTER, &'static str>,
    #[cfg(windows)]
    disk_read: Result<PDH_HCOUNTER, &'static str>,
    #[cfg(windows)]
    disk_write: Result<PDH_HCOUNTER, &'static str>,
}

/// Groups and fields from one collection.
pub(crate) struct PdhGroups {
    pub(crate) gpu: AvailabilityV1<GpuV1>,
    pub(crate) thermal: AvailabilityV1<ThermalV1>,
    pub(crate) cpu_frequency_mhz: Option<u32>,
    pub(crate) disk_activity: AvailabilityV1<DiskActivityV1>,
}

impl PdhProbe {
    /// Open the query and collect a baseline for the rate counter.
    pub(crate) fn open_primed() -> Self {
        let probe = Self::open();
        #[cfg(windows)]
        let _ = probe.collect();
        probe
    }

    /// Collect once and read every counter. Rate counters cover the time since
    /// the previous collection.
    pub(crate) fn sample(&self, sampled_at_unix_ms: u64) -> PdhGroups {
        #[cfg(windows)]
        {
            let collected = self.collect();
            let array = |counter: Result<PDH_HCOUNTER, &'static str>| match collected {
                Ok(()) => counter.and_then(read_counter),
                Err(reason) => counter.and(Err(reason)),
            };
            let single = |counter: Result<PDH_HCOUNTER, &'static str>| match collected {
                Ok(()) => counter.and_then(read_value),
                Err(reason) => counter.and(Err(reason)),
            };
            PdhGroups {
                gpu: gpu_availability(array(self.gpu), sampled_at_unix_ms),
                thermal: thermal_availability(array(self.thermal), sampled_at_unix_ms),
                cpu_frequency_mhz: frequency_mhz(
                    single(self.cpu_frequency),
                    single(self.cpu_performance),
                ),
                disk_activity: disk_activity_availability(
                    single(self.disk_read),
                    single(self.disk_write),
                    sampled_at_unix_ms,
                ),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = sampled_at_unix_ms;
            PdhGroups {
                gpu: AvailabilityV1::unsupported("platform_unsupported"),
                thermal: AvailabilityV1::unsupported("platform_unsupported"),
                cpu_frequency_mhz: None,
                disk_activity: AvailabilityV1::unsupported("platform_unsupported"),
            }
        }
    }

    #[cfg(windows)]
    fn open() -> Self {
        use windows_sys::Win32::System::Performance::PdhOpenQueryW;

        let mut query = std::ptr::null_mut();
        // SAFETY: a null data source selects live data; `query` is writable.
        let status = unsafe { PdhOpenQueryW(std::ptr::null(), 0, &mut query) };
        if status != 0 || query.is_null() {
            return Self {
                query: std::ptr::null_mut(),
                gpu: Err("pdh_unavailable"),
                thermal: Err("pdh_unavailable"),
                cpu_frequency: Err("pdh_unavailable"),
                cpu_performance: Err("pdh_unavailable"),
                disk_read: Err("pdh_unavailable"),
                disk_write: Err("pdh_unavailable"),
            };
        }
        Self {
            query,
            gpu: add_counter(query, GPU_ENGINE_COUNTER),
            thermal: add_counter(query, THERMAL_ZONE_COUNTER),
            cpu_frequency: add_counter(query, CPU_FREQUENCY_COUNTER),
            cpu_performance: add_counter(query, CPU_PERFORMANCE_COUNTER),
            disk_read: add_counter(query, DISK_READ_COUNTER),
            disk_write: add_counter(query, DISK_WRITE_COUNTER),
        }
    }

    #[cfg(not(windows))]
    fn open() -> Self {
        Self {}
    }

    #[cfg(windows)]
    fn collect(&self) -> Result<(), &'static str> {
        use windows_sys::Win32::System::Performance::PdhCollectQueryData;

        if self.query.is_null() {
            return Err("pdh_unavailable");
        }
        // SAFETY: `query` is an open PDH query owned by this probe.
        let status = unsafe { PdhCollectQueryData(self.query) };
        if status == 0 {
            Ok(())
        } else {
            Err("collect_failed")
        }
    }
}

#[cfg(windows)]
impl Drop for PdhProbe {
    fn drop(&mut self) {
        use windows_sys::Win32::System::Performance::PdhCloseQuery;

        if !self.query.is_null() {
            // SAFETY: `query` is open and closed exactly once here.
            unsafe { PdhCloseQuery(self.query) };
        }
    }
}

#[cfg(windows)]
fn add_counter(query: PDH_HQUERY, path: &str) -> Result<PDH_HCOUNTER, &'static str> {
    use windows_sys::Win32::System::Performance::PdhAddEnglishCounterW;

    let wide = path.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let mut counter = std::ptr::null_mut();
    // SAFETY: `wide` is NUL-terminated and `counter` is writable.
    let status = unsafe { PdhAddEnglishCounterW(query, wide.as_ptr(), 0, &mut counter) };
    if status == 0 && !counter.is_null() {
        Ok(counter)
    } else {
        Err("counter_missing")
    }
}

#[cfg(windows)]
fn read_counter(counter: PDH_HCOUNTER) -> CounterRead {
    use windows_sys::Win32::System::Performance::{
        PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_NO_INSTANCE, PDH_CSTATUS_VALID_DATA,
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA, PDH_NO_DATA,
        PdhGetFormattedCounterArrayW,
    };
    /// `PDH_FMT_NOCAP100`: GPU engine sums may pass 100 before the clamp.
    const PDH_FMT_NOCAP100: u32 = 0x0000_8000;
    const FORMAT: u32 = PDH_FMT_DOUBLE | PDH_FMT_NOCAP100;

    for _ in 0..3 {
        let mut size = 0_u32;
        let mut count = 0_u32;
        // SAFETY: a null buffer with size 0 asks PDH for the required size.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                FORMAT,
                &mut size,
                &mut count,
                std::ptr::null_mut(),
            )
        };
        match status {
            PDH_MORE_DATA => {}
            0 | PDH_NO_DATA | PDH_CSTATUS_NO_INSTANCE => return Ok(Vec::new()),
            _ => return Err("counter_read_failed"),
        }
        let words = (size as usize).div_ceil(std::mem::size_of::<u64>());
        let mut buffer = vec![0_u64; words.max(1)];
        // SAFETY: `buffer` is 8-byte aligned and at least `size` bytes long.
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                counter,
                FORMAT,
                &mut size,
                &mut count,
                buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
            )
        };
        if status == PDH_MORE_DATA {
            continue;
        }
        if status != 0 {
            return Err("counter_read_failed");
        }
        // SAFETY: PDH wrote `count` items at the start of `buffer`.
        let items = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            )
        };
        let mut samples = Vec::with_capacity(items.len());
        for item in items {
            let valid = item.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA
                || item.FmtValue.CStatus == PDH_CSTATUS_NEW_DATA;
            if !valid || item.szName.is_null() {
                continue;
            }
            // SAFETY: `szName` is a NUL-terminated string inside `buffer`.
            let name = unsafe { wide_to_string(item.szName) };
            // SAFETY: PDH_FMT_DOUBLE selects the `doubleValue` member.
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            samples.push(CounterSample { name, value });
        }
        if samples.is_empty() && !items.is_empty() {
            return Err("no_valid_data");
        }
        return Ok(samples);
    }
    Err("counter_read_failed")
}

#[cfg(windows)]
fn read_value(counter: PDH_HCOUNTER) -> ValueRead {
    use windows_sys::Win32::System::Performance::{
        PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
        PdhGetFormattedCounterValue,
    };
    /// `PDH_FMT_NOCAP100`: `% Processor Performance` routinely exceeds 100.
    const PDH_FMT_NOCAP100: u32 = 0x0000_8000;

    // SAFETY: zeroed PDH_FMT_COUNTERVALUE is a valid out-parameter.
    let mut value: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
    let mut kind = 0_u32;
    // SAFETY: `counter` belongs to this probe's open query; both out-pointers
    // are writable locals.
    let status = unsafe {
        PdhGetFormattedCounterValue(
            counter,
            PDH_FMT_DOUBLE | PDH_FMT_NOCAP100,
            &mut kind,
            &mut value,
        )
    };
    if status != 0 {
        return Err("counter_read_failed");
    }
    if value.CStatus != PDH_CSTATUS_VALID_DATA && value.CStatus != PDH_CSTATUS_NEW_DATA {
        return Err("no_valid_data");
    }
    // SAFETY: PDH_FMT_DOUBLE selects the `doubleValue` member.
    Ok(unsafe { value.Anonymous.doubleValue })
}

#[cfg(windows)]
unsafe fn wide_to_string(pointer: *const u16) -> String {
    let mut len = 0_usize;
    // SAFETY: the caller guarantees a NUL-terminated UTF-16 string.
    while unsafe { *pointer.add(len) } != 0 {
        len += 1;
    }
    // SAFETY: `len` units before the terminator are readable.
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, len) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str, value: f64) -> CounterSample {
        CounterSample {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn gpu_sums_engine_type_and_keeps_the_busiest_type_per_adapter() {
        let samples = [
            sample(
                "pid_10_luid_0x00000000_0x0000C0B6_phys_0_eng_0_engtype_3D",
                12.5,
            ),
            sample(
                "pid_20_luid_0x00000000_0x0000C0B6_phys_0_eng_0_engtype_3D",
                20.0,
            ),
            sample(
                "pid_10_luid_0x00000000_0x0000C0B6_phys_0_eng_3_engtype_VideoDecode",
                40.0,
            ),
            sample(
                "pid_30_luid_0x00000000_0x0000D1A0_phys_0_eng_1_engtype_Copy",
                1.25,
            ),
        ];
        let gpu = gpu_from_samples(&samples).expect("adapters");
        assert_eq!(
            gpu.adapters,
            vec![
                GpuAdapterV1 {
                    adapter_id: "luid_0x00000000_0x0000C0B6_phys_0".into(),
                    utilization_basis_points: 4_000,
                    name: None,
                },
                GpuAdapterV1 {
                    adapter_id: "luid_0x00000000_0x0000D1A0_phys_0".into(),
                    utilization_basis_points: 125,
                    name: None,
                },
            ]
        );
    }

    #[test]
    fn gpu_sum_is_clamped_to_one_hundred_percent() {
        let samples = [
            sample("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 70.0),
            sample("pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 55.0),
        ];
        let gpu = gpu_from_samples(&samples).expect("adapter");
        assert_eq!(gpu.adapters[0].utilization_basis_points, 10_000);
    }

    #[test]
    fn gpu_idle_engines_are_a_measured_zero_but_no_instance_is_unavailable() {
        let idle = [sample("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 0.0)];
        assert_eq!(
            gpu_from_samples(&idle).expect("measured idle").adapters[0].utilization_basis_points,
            0
        );
        assert_eq!(gpu_from_samples(&[]), None);
        let malformed = [
            sample("no adapter token", 50.0),
            sample("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", f64::NAN),
            sample("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", -1.0),
        ];
        assert_eq!(gpu_from_samples(&malformed), None);
        assert!(matches!(
            gpu_availability(Ok(Vec::new()), 1),
            AvailabilityV1::Unavailable { ref reason_code, .. } if reason_code == "no_instances"
        ));
    }

    #[test]
    fn missing_counters_are_unavailable_with_a_reason_and_never_zero() {
        for reason in ["pdh_unavailable", "counter_missing", "no_valid_data"] {
            let gpu = gpu_availability(Err(reason), 1);
            let thermal = thermal_availability(Err(reason), 1);
            assert_eq!(gpu, AvailabilityV1::unavailable(reason));
            assert_eq!(thermal, AvailabilityV1::unavailable(reason));
            assert!(!gpu.has_value());
            assert!(!thermal.has_value());
            let encoded = serde_json::to_string(&gpu).unwrap();
            assert!(!encoded.contains("utilization_basis_points"));
        }
    }

    #[test]
    fn thermal_converts_kelvin_to_tenths_celsius_and_drops_zero_kelvin() {
        let samples = [
            sample(r"\_TZ.TZ00", 301.0),
            sample(r"\_SB.ECTZ", 0.0),
            sample(r"\_TZ.CPUZ", 353.15),
        ];
        let thermal = thermal_from_samples(&samples).expect("valid zones");
        assert_eq!(
            thermal.zones,
            vec![
                ThermalZoneV1 {
                    zone_id: r"\_TZ.CPUZ".into(),
                    temperature_tenths_celsius: 800,
                },
                ThermalZoneV1 {
                    zone_id: r"\_TZ.TZ00".into(),
                    temperature_tenths_celsius: 279,
                },
            ]
        );
        assert_eq!(thermal_from_samples(&[sample(r"\_SB.ECTZ", 0.0)]), None);
        assert!(matches!(
            thermal_availability(Ok(vec![sample(r"\_SB.ECTZ", 0.0)]), 1),
            AvailabilityV1::Unavailable { ref reason_code, .. } if reason_code == "no_instances"
        ));
    }

    #[test]
    fn counter_paths_are_the_english_paths() {
        assert_eq!(GPU_ENGINE_COUNTER, r"\GPU Engine(*)\Utilization Percentage");
        assert_eq!(
            THERMAL_ZONE_COUNTER,
            r"\Thermal Zone Information(*)\Temperature"
        );
        assert_eq!(
            CPU_FREQUENCY_COUNTER,
            r"\Processor Information(_Total)\Processor Frequency"
        );
        assert_eq!(
            CPU_PERFORMANCE_COUNTER,
            r"\Processor Information(_Total)\% Processor Performance"
        );
        assert_eq!(
            DISK_READ_COUNTER,
            r"\PhysicalDisk(_Total)\Disk Read Bytes/sec"
        );
        assert_eq!(
            DISK_WRITE_COUNTER,
            r"\PhysicalDisk(_Total)\Disk Write Bytes/sec"
        );
    }

    #[test]
    fn frequency_is_nominal_times_performance_and_missing_is_none_not_zero() {
        // Task Manager formula: 2300 MHz nominal at 196.5 % performance.
        assert_eq!(frequency_mhz(Ok(2_300.0), Ok(196.5)), Some(4_520));
        assert_eq!(frequency_mhz(Ok(2_300.0), Ok(100.0)), Some(2_300));
        for (nominal, performance) in [
            (Err("counter_missing"), Ok(100.0)),
            (Ok(2_300.0), Err("no_valid_data")),
            (Ok(0.0), Ok(100.0)),
            (Ok(2_300.0), Ok(0.0)),
            (Ok(f64::NAN), Ok(100.0)),
            (Ok(2_300.0), Ok(-5.0)),
        ] {
            assert_eq!(frequency_mhz(nominal, performance), None);
        }
    }

    #[test]
    fn disk_activity_is_measured_bytes_or_unavailable_with_a_reason() {
        let AvailabilityV1::Available { value, .. } =
            disk_activity_availability(Ok(1_040_000.4), Ok(0.0), 7)
        else {
            panic!("both counters read");
        };
        assert_eq!(value.read_bytes_per_second, 1_040_000);
        assert_eq!(value.write_bytes_per_second, 0);
        assert_eq!(
            disk_activity_availability(Err("counter_missing"), Ok(1.0), 7),
            AvailabilityV1::unavailable("counter_missing")
        );
        assert_eq!(
            disk_activity_availability(Ok(1.0), Err("collect_failed"), 7),
            AvailabilityV1::unavailable("collect_failed")
        );
        assert_eq!(
            disk_activity_availability(Ok(-1.0), Ok(1.0), 7),
            AvailabilityV1::unavailable("no_valid_data")
        );
        assert_eq!(
            disk_activity_availability(Ok(1.0), Ok(f64::INFINITY), 7),
            AvailabilityV1::unavailable("no_valid_data")
        );
        let encoded = serde_json::to_string(&disk_activity_availability(
            Err("pdh_unavailable"),
            Ok(1.0),
            7,
        ))
        .unwrap();
        assert!(!encoded.contains("bytes_per_second"));
    }
}
