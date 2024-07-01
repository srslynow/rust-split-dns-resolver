use std::net::IpAddr;

pub fn split_string(ip_list_string : String) -> Vec<String> {
    ip_list_string
        .split(",")
        .map(|s| String::from(s.trim()))
        .collect::<Vec<String>>()
}

pub fn parse_ipaddrs(ip_list_string : Vec<String>) -> Vec<IpAddr> {
    ip_list_string
        .iter()
        .map(|i| (i.parse::<IpAddr>()).unwrap())
        .collect::<Vec<IpAddr>>()
}

pub fn parse_ipaddrs_from_str(ip_list_string : String) -> Vec<IpAddr> {
    parse_ipaddrs(split_string(ip_list_string))
}
