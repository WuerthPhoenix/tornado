use criterion::Criterion;
use regex::Regex;

pub fn bench(c: &mut Criterion) {
    let texts = &vec![
        "MWRM2-NMS-MIB::netmasterAlarmNeIpv4Address.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmNeIpv6Address.201476692",
        "MWRM2-NMS-MIB::netmasterAlarm.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmStatus.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmSomethingElse.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmFreddieMercury.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmRogerTaylor.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmBrainMay.201476692",
        "MWRM2-NMS-MIB::netmasterAlarmJohnDeacon.201476692",
        "MWRM2-NMS-MIB::netmasterLedZeppelin.201476692",
        "MWRM2-NMS-MIB::netmasterLedZeppelin",
    ];

    let prefix = "MWRM2-NMS-MIB::netmasterAlarmNeIpv4Address";
    let start_with_regex = Regex::new(&format!(r#"{}\."#, prefix)).unwrap();

    c.bench_function("String StartWith - Regex", |b| {
        b.iter(|| {
            let mut found = 0;
            for text in texts {
                if start_with_regex.is_match(text) {
                    found += 1;
                }
            }
            assert!(found > 0)
        })
    });

    c.bench_function("String StartWith - Native", |b| {
        b.iter(|| {
            let mut found = 0;
            for text in texts {
                if text.starts_with(prefix) {
                    found += 1;
                }
            }
            assert!(found > 0)
        })
    });
}
