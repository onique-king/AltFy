use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, net::{SocketAddr, TcpStream, ToSocketAddrs}, path::PathBuf, process::{Child, Command, Stdio}, sync::{Mutex, OnceLock}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Emitter, Manager};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CHROME_WINDOWS_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const SAFARI_MACOS_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15";
const SAFARI_IOS_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1";
const FIREFOX_WINDOWS_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const CHROME_ANDROID_UA: &str = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36";

fn browser_user_agent(value: Option<&str>) -> &'static str {
    match value.unwrap_or("chrome_windows") {
        "safari_macos" => SAFARI_MACOS_UA,
        "safari_ios" => SAFARI_IOS_UA,
        "firefox_windows" => FIREFOX_WINDOWS_UA,
        "chrome_android" => CHROME_ANDROID_UA,
        _ => CHROME_WINDOWS_UA,
    }
}

fn diagnostic_probe(url: &str) -> (bool, u64) {
    let started = Instant::now();
    let mut command = Command::new("curl.exe");
    command.args(["-s", "-L", "-A", CHROME_WINDOWS_UA, "-o", "nul", "-w", "%{http_code}", "--max-time", "4", url])
        .stdin(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = command.output();
    let latency_ms = started.elapsed().as_millis() as u64;
    let status_code = output.ok().filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|code| code.trim().parse::<u16>().ok());
    (status_code.is_some_and(|code| (200..=399).contains(&code)), latency_ms)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server { id:String, flag:String, name:String, protocol:String, ping:i64, group:String, address:String, port:u16, key:String, #[serde(default)] public_key:Option<String>, #[serde(default)] short_id:Option<String>, source:Option<String>, config:Value }
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Subscription { id:String, name:String, source:String, upload:u64, download:u64, total:u64, expire:u64, #[serde(default)] description:String }
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State { servers:Vec<Server>, subscriptions:Vec<Subscription>, #[serde(default, rename = "selected_server_id", alias = "selectedId")] selected_id:Option<String> }
static CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
fn child() -> &'static Mutex<Option<Child>> { CHILD.get_or_init(|| Mutex::new(None)) }
fn state_path(app:&AppHandle)->Result<PathBuf,String>{let p=app.path().app_local_data_dir().map_err(|e|e.to_string())?.join("vpn-state.json");if let Some(d)=p.parent(){fs::create_dir_all(d).map_err(|e|e.to_string())?;}Ok(p)}
fn load(app:&AppHandle)->Result<State,String>{let p=state_path(app)?;if !p.exists(){return Ok(State::default())}serde_json::from_slice(&fs::read(p).map_err(|e|e.to_string())?).map_err(|e|e.to_string())}
fn save(app:&AppHandle,s:&State)->Result<(),String>{fs::write(state_path(app)?,serde_json::to_vec_pretty(s).map_err(|e|e.to_string())?).map_err(|e|e.to_string())}
fn uid()->String{format!("vpn-{}",SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis())}
pub(crate) fn device_hwid(app:&AppHandle)->String{
 #[cfg(windows)]
 let identity=Command::new("reg").args(["query","HKLM\\SOFTWARE\\Microsoft\\Cryptography","/v","MachineGuid"]).output().ok().and_then(|o|String::from_utf8(o.stdout).ok()).and_then(|output|output.lines().find(|line|line.contains("MachineGuid")).and_then(|line|line.split_whitespace().last().map(str::to_owned)));
 #[cfg(not(windows))]
 let identity=None;
 let fallback=app.path().app_local_data_dir().map(|path|path.to_string_lossy().into_owned()).unwrap_or_else(|_|std::env::var("COMPUTERNAME").or_else(|_|std::env::var("HOSTNAME")).unwrap_or_default());
 format!("{:x}",sha2::Sha256::digest(identity.unwrap_or(fallback).as_bytes()))
}
fn dec(s:&str)->String{urlencoding::decode(s).map(|x|x.into_owned()).unwrap_or_else(|_|s.into())}
fn profile_title(value:&str,fallback:&str)->String{let value=value.trim();if value.get(..7).is_some_and(|prefix|prefix.eq_ignore_ascii_case("base64:")){let encoded=&value[7..];return general_purpose::STANDARD.decode(encoded).or_else(|_|general_purpose::STANDARD_NO_PAD.decode(encoded)).ok().and_then(|bytes|String::from_utf8(bytes).ok()).map(|title|title.trim().to_owned()).filter(|title|!title.is_empty()).unwrap_or_else(||fallback.into())}value.into()}
fn decode_base64_text(value:&str)->Option<String>{let encoded=value.trim().strip_prefix('\u{feff}').unwrap_or(value).chars().filter(|character|!character.is_whitespace()).collect::<String>();if encoded.is_empty(){return None}let decoded=general_purpose::STANDARD.decode(&encoded).or_else(|_|general_purpose::STANDARD_NO_PAD.decode(&encoded)).or_else(|_|general_purpose::URL_SAFE.decode(&encoded)).or_else(|_|general_purpose::URL_SAFE_NO_PAD.decode(&encoded)).ok().and_then(|bytes|String::from_utf8(bytes).ok()).map(|text|text.trim().to_owned()).filter(|text|!text.is_empty());decoded.filter(|text|text.contains("://")||text.trim_start().starts_with('{')||text.trim_start().starts_with('[')||text.contains("proxies:"))}
fn announce_description(value:&str)->String{let value=value.trim();if value.get(..7).is_some_and(|prefix|prefix.eq_ignore_ascii_case("base64:")){decode_base64_text(&value[7..]).unwrap_or_default()}else{value.to_owned()}}
fn singbox_server(value:&Value,group:&str)->Option<Server>{let protocol=value["type"].as_str()?.to_ascii_lowercase();if !matches!(protocol.as_str(),"vless"|"vmess"|"trojan"|"shadowsocks"|"hysteria2"|"hysteria"|"wireguard"){return None}let address=value["server"].as_str()?.to_owned();let port=value["server_port"].as_u64().or_else(||value["server_port"].as_str().and_then(|port|port.parse().ok())).and_then(|port|u16::try_from(port).ok())?;if address.is_empty()||port==0{return None}let name=value["tag"].as_str().filter(|name|!name.is_empty()).unwrap_or(&address).to_owned();let mut config=value.clone();config["tag"]=json!("proxy");Some(Server{id:uid(),flag:country_flag(&name),name,protocol:protocol.to_uppercase(),ping:0,group:group.into(),address,port,key:serde_json::to_string(value).unwrap_or_default(),public_key:reality_keys(&config).0,short_id:reality_keys(&config).1,source:None,config})}
fn xray_server(profile:&Value,group:&str)->Option<Server>{let remarks=profile["remarks"].as_str().unwrap_or("Xray server").to_owned();let outbound=profile["outbounds"].as_array()?.iter().find(|outbound|matches!(outbound["protocol"].as_str().unwrap_or("").to_ascii_lowercase().as_str(),"vless"|"vmess"|"trojan"|"shadowsocks"))?;let protocol=outbound["protocol"].as_str()?.to_ascii_lowercase();let peer=outbound["settings"]["vnext"].as_array().and_then(|peers|peers.first()).or_else(||outbound["settings"]["servers"].as_array().and_then(|peers|peers.first()))?;let address=peer["address"].as_str()?.to_owned();let port=peer["port"].as_u64().or_else(||peer["port"].as_str().and_then(|value|value.parse().ok())).and_then(|value|u16::try_from(value).ok())?;if port==0{return None}let user=peer["users"].as_array().and_then(|users|users.first()).unwrap_or(&Value::Null);let stream=&outbound["streamSettings"];let security=stream["security"].as_str().unwrap_or("none");let tls_settings=if security=="reality"{&stream["realitySettings"]}else{&stream["tlsSettings"]};let mut tls=if security=="none"{json!({"enabled":false})}else{json!({"enabled":true,"server_name":tls_settings["serverName"],"insecure":tls_settings["allowInsecure"]==true})};if security=="reality"{tls["reality"]=json!({"enabled":true,"public_key":tls_settings["publicKey"],"short_id":tls_settings["shortId"]});}if let Some(fingerprint)=tls_settings["fingerprint"].as_str(){tls["utls"]=json!({"enabled":true,"fingerprint":fingerprint});}if security!="reality" {if let Some(alpn)=tls_settings["alpn"].as_array(){tls["alpn"]=json!(alpn);}}let transport=match stream["network"].as_str().unwrap_or("tcp"){"ws"=>json!({"type":"ws","path":stream["wsSettings"]["path"],"headers":stream["wsSettings"]["headers"]}),"grpc"=>json!({"type":"grpc","service_name":stream["grpcSettings"]["serviceName"]}),_=>Value::Null};let config=match protocol.as_str(){"vless"=>json!({"type":"vless","tag":"proxy","server":address,"server_port":port,"uuid":user["id"],"flow":user["flow"],"tls":tls,"transport":transport}),"vmess"=>json!({"type":"vmess","tag":"proxy","server":address,"server_port":port,"uuid":user["id"],"security":user["security"].as_str().unwrap_or("auto"),"alter_id":user["alterId"].as_u64().unwrap_or(0),"tls":tls,"transport":transport}),"trojan"=>json!({"type":"trojan","tag":"proxy","server":address,"server_port":port,"password":user["password"],"tls":tls,"transport":transport}),"shadowsocks"=>json!({"type":"shadowsocks","tag":"proxy","server":address,"server_port":port,"method":user["method"],"password":user["password"]}),_=>return None};Some(Server{id:uid(),flag:country_flag(&remarks),name:remarks,protocol:protocol.to_uppercase(),ping:0,group:group.into(),address,port,key:serde_json::to_string(profile).unwrap_or_default(),public_key:reality_keys(&config).0,short_id:reality_keys(&config).1,source:None,config})}
fn subscription_servers(content:&str,group:&str)->Vec<Server>{if let Ok(value)=serde_json::from_str::<Value>(content){if let Some(profiles)=value.as_array(){let servers=profiles.iter().filter_map(|profile|xray_server(profile,group)).collect::<Vec<_>>();if !servers.is_empty(){return servers}}if let Some(outbounds)=value["outbounds"].as_array(){let servers=outbounds.iter().filter_map(|outbound|singbox_server(outbound,group)).collect::<Vec<_>>();if !servers.is_empty(){return servers}}}let servers=content.lines().filter_map(|line|{let line=line.trim().trim_start_matches('\u{feff}');if line.is_empty()||line.starts_with('#')||line.starts_with("//"){return None}let line=line.trim_matches(|c|c=='\''||c=='"');let (scheme,_)=line.split_once("://")?;matches!(scheme.to_ascii_lowercase().as_str(),"vless"|"vmess"|"trojan"|"ss"|"hy2"|"hysteria2"|"wg"|"wireguard").then(||parse_link(line,group).ok()).flatten()}).collect::<Vec<_>>();if !servers.is_empty(){return servers}let mut in_proxies=false;let mut proxy=serde_json::Map::new();let mut proxies=Vec::new();for line in content.lines(){let trimmed=line.trim();if trimmed=="proxies:"{in_proxies=true;continue}if !in_proxies{continue}if !line.starts_with(' ')&&!line.starts_with('\t'){break}if let Some(item)=trimmed.strip_prefix("- "){if !proxy.is_empty(){proxies.push(Value::Object(std::mem::take(&mut proxy)));}if let Some((key,value))=item.split_once(':'){proxy.insert(key.trim().into(),json!(value.trim().trim_matches(['\'','"'])));}}else if let Some((key,value))=trimmed.split_once(':'){let value=value.trim().trim_matches(['\'','"']);if !value.is_empty(){proxy.insert(key.trim().into(),json!(value));}}}if !proxy.is_empty(){proxies.push(Value::Object(proxy));}proxies.into_iter().filter_map(|mut proxy|{let protocol=proxy.get("type")?.as_str()?.to_ascii_lowercase();let name=proxy.get("name").and_then(Value::as_str).unwrap_or("Proxy").to_owned();proxy["tag"]=json!(name);let port=proxy.get("port").cloned().unwrap_or(Value::Null);let singbox_protocol=match protocol.as_str(){"ss"=>"shadowsocks","hy2"=>"hysteria2",_=>protocol.as_str()};let server=proxy.get("server").cloned().unwrap_or(Value::Null);proxy.as_object_mut()?.insert("server_port".into(),port);proxy.as_object_mut()?.insert("type".into(),json!(singbox_protocol));proxy.as_object_mut()?.insert("server".into(),server);singbox_server(&proxy,group)}).collect()}
fn query(url:&url::Url)->std::collections::HashMap<String,String>{url.query_pairs().map(|(k,v)|(k.into_owned(),v.into_owned())).collect()}
fn reality_keys(value:&Value)->(Option<String>,Option<String>){let reality=&value["tls"]["reality"];let public_key=reality["public_key"].as_str().or_else(||value["public_key"].as_str()).filter(|value|!value.is_empty()).map(str::to_owned);let short_id=reality["short_id"].as_str().or_else(||value["short_id"].as_str()).filter(|value|!value.is_empty()).map(str::to_owned);(public_key,short_id)}
fn link_tls(protocol:&str,address:&str,q:&std::collections::HashMap<String,String>)->Value{let security=q.get("security").map(String::as_str).unwrap_or(if protocol=="ss"||protocol=="wireguard"||protocol=="wg"{"none"}else{"tls"});if security=="none"{return json!({"enabled":false})}let mut tls=json!({"enabled":true,"server_name":q.get("sni").or(q.get("serverName")).cloned().filter(|value|!value.is_empty()).unwrap_or_else(||address.to_owned()),"insecure":q.get("allowInsecure").is_some_and(|value|value=="1")});if security=="reality"{tls["reality"]=json!({"enabled":true,"public_key":q.get("pbk").or(q.get("publicKey")).cloned().unwrap_or_default(),"short_id":q.get("sid").cloned().unwrap_or_default()});}if security=="tls"||security=="reality"{tls["utls"]=json!({"enabled":true,"fingerprint":q.get("fp").filter(|value|!value.is_empty()).cloned().unwrap_or_else(||"chrome".into())});}if security!="reality" {if let Some(alpn)=q.get("alpn"){tls["alpn"]=json!(alpn.split(',').collect::<Vec<_>>());}}tls}
fn parse_link(line:&str,group:&str)->Result<Server,String>{
 let (scheme,rest)=line.split_once("://").ok_or("Некорректная ссылка")?;let protocol=scheme.to_ascii_lowercase();let address:String;let port:u16;let config:Value;let mut name:String;
 if protocol=="vmess" {let encoded=rest.trim();let b=general_purpose::STANDARD.decode(encoded).or_else(|_|general_purpose::STANDARD_NO_PAD.decode(encoded)).or_else(|_|general_purpose::URL_SAFE.decode(encoded)).or_else(|_|general_purpose::URL_SAFE_NO_PAD.decode(encoded)).map_err(|_|"Некорректный VMess Base64")?;let v:Value=serde_json::from_slice(&b).map_err(|e|e.to_string())?;address=v["add"].as_str().unwrap_or("").into();port=v["port"].as_str().and_then(|x|x.parse().ok()).or(v["port"].as_u64().map(|x|x as u16)).unwrap_or(443);name=v["ps"].as_str().unwrap_or("VMess").into();config=json!({"type":"vmess","tag":"proxy","server":address,"server_port":port,"uuid":v["id"],"security":v["scy"].as_str().unwrap_or("auto"),"alter_id":v["aid"].as_str().and_then(|x|x.parse::<u32>().ok()).unwrap_or(0),"tls":if v["tls"].as_str()==Some("tls"){json!({"enabled":true,"server_name":v["sni"]})}else{json!({"enabled":false})},"transport":transport(v["net"].as_str().unwrap_or(""),v["path"].as_str().unwrap_or(""),v["host"].as_str().unwrap_or(""))}); }
 else {let u=url::Url::parse(line).map_err(|e|e.to_string())?;address=u.host_str().unwrap_or("").to_string();port=u.port().unwrap_or(match protocol.as_str(){"ss"=>8388,"wireguard"|"wg"=>51820,_=>443});name=dec(u.fragment().unwrap_or(""));let q=query(&u);let user=dec(u.username());let password=dec(u.password().unwrap_or(""));let tls=link_tls(&protocol,&address,&q);let transport_kind=q.get("type").map(String::as_str).unwrap_or("");let tr=if transport_kind=="tcp"{q.get("headerType").filter(|kind|kind.as_str()=="http").map(|_|json!({"type":"http","host":q.get("host").map(|host|host.split(',').collect::<Vec<_>>()).unwrap_or_default()})).unwrap_or(Value::Null)}else{transport(transport_kind,q.get("path").map(String::as_str).unwrap_or(""),q.get("host").map(String::as_str).unwrap_or(""))};
 config=match protocol.as_str(){"vless"=>json!({"type":"vless","tag":"proxy","server":address,"server_port":port,"uuid":user,"flow":q.get("flow").cloned().unwrap_or_default(),"tls":if tls["enabled"]==true{tls}else{json!({"enabled":false})},"transport":if tr.is_null(){Value::Null}else{tr}}),"trojan"=>json!({"type":"trojan","tag":"proxy","server":address,"server_port":port,"password":password,"tls":tls,"transport":tr}),"ss"=>{let credential=if !user.is_empty(){user.clone()}else{rest.split('@').next().unwrap_or("").to_string()};let decoded=general_purpose::STANDARD.decode(&credential).or_else(|_|general_purpose::STANDARD_NO_PAD.decode(&credential)).or_else(|_|general_purpose::URL_SAFE.decode(&credential)).or_else(|_|general_purpose::URL_SAFE_NO_PAD.decode(&credential)).ok().and_then(|x|String::from_utf8(x).ok()).unwrap_or(credential);let (method,pw)=decoded.split_once(':').unwrap_or(("chacha20-ietf-poly1305",decoded.as_str()));json!({"type":"shadowsocks","tag":"proxy","server":address,"server_port":port,"method":method,"password":pw})},"hy2"|"hysteria2"=>json!({"type":"hysteria2","tag":"proxy","server":address,"server_port":port,"password":password,"tls":tls}),"wireguard"|"wg"=>json!({"type":"wireguard","tag":"proxy","server":address,"server_port":port,"private_key":password,"peer_public_key":q.get("publickey"),"local_address":["172.16.0.2/32"],"mtu":1280}),_=>return Err("Неподдерживаемый протокол".into())};}
 if address.is_empty()||port==0{return Err("В ссылке отсутствует адрес или порт".into())}if name.is_empty(){name=format!("{} · {}",protocol.to_uppercase(),address)}let flag=country_flag(&name);Ok(Server{id:uid(),flag,name,protocol:protocol.to_uppercase(),ping:0,group:group.into(),address,port,key:line.into(),public_key:reality_keys(&config).0,short_id:reality_keys(&config).1,source:None,config})
}
fn transport(kind:&str,path:&str,host:&str)->Value{match kind{"ws"=>{let mut value=json!({"type":"ws","path":path});if !host.is_empty(){value["headers"]=json!({"Host":host});}value},"grpc"=>json!({"type":"grpc","service_name":path.trim_start_matches('/') }),"httpupgrade"=>{let mut value=json!({"type":"httpupgrade","path":path});if !host.is_empty(){value["host"]=json!(host);}value},_=>Value::Null}}
fn country_flag(n:&str)->String{for (key,flag) in [("Netherlands","nl"),("Нидерланд","nl"),("Germany","de"),("Герман","de"),("Finland","fi"),("Финлян","fi"),("United States","us"),("США","us"),("Japan","jp"),("Япони","jp"),("UK","gb"),("Switzerland","ch")]{if n.to_lowercase().contains(&key.to_lowercase()){return flag.into()}}"🌐".into()}
#[tauri::command]
pub async fn import_vpn_source(app:AppHandle,input:String)->Result<Value,String>{let input=input.trim();let mut s=load(&app)?;if input.get(..8).is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))||input.get(..7).is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://")){let reqwest_client=reqwest::Client::builder().timeout(Duration::from_secs(45)).build().map_err(|e|e.to_string())?;let user_agent=browser_user_agent(crate::settings::settings(&app)["userAgent"].as_str());let req=reqwest_client.get(input).header("User-Agent",user_agent).header("X-HWID",device_hwid(&app)).header("X-Device-OS","Windows").header("X-Ver-OS",std::env::var("OS").unwrap_or_else(|_|"Windows 10".into())).header("X-Device-Model",std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_|"PC".into())).header("X-Device-Type","desktop");let resp=req.send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?;let response_headers=resp.headers().clone();let info=response_headers.iter().find(|(k,_)|k.as_str().eq_ignore_ascii_case("subscription-userinfo")).and_then(|(_,v)|v.to_str().ok()).unwrap_or("").to_string();let raw_response_len=resp.content_length().unwrap_or(0);let announce=response_headers.iter().find(|(k,_)|k.as_str().eq_ignore_ascii_case("announce")).and_then(|(_,v)|v.to_str().ok()).map(str::to_owned);let header_name=response_headers.iter().find(|(name,_)|name.as_str().eq_ignore_ascii_case("profile-title")).and_then(|(_,value)|value.to_str().ok()).map(str::to_owned).or_else(||resp.headers().get("content-disposition").and_then(|v|v.to_str().ok()).and_then(|v|v.split(';').find_map(|part|part.trim().strip_prefix("filename=").or_else(||part.trim().strip_prefix("filename*=UTF-8''")))).map(|v|v.trim_matches(|c|c=='\"'||c=='\'').to_owned()));let body=resp.text().await.map_err(|e|e.to_string())?;
let response_len=body.len().max(raw_response_len as usize);
let decoded=decode_base64_text(&body).unwrap_or_else(||body.clone());let parsed_count=subscription_servers(&decoded,"").len();let _=app.emit("engine-log",format!("VPN subscription response received: {response_len} bytes, {parsed_count} servers recognized"));let info=if info.is_empty(){decoded.lines().take(5).map(str::trim).find(|line|line.to_ascii_lowercase().contains("upload=")&&line.to_ascii_lowercase().contains("download=")).map(|line|line.trim_start_matches(|c|c=='#'||c==' ').to_string()).unwrap_or_default()}else{info};let fallback_name=url::Url::parse(input).ok().and_then(|u|u.host_str().map(str::to_owned)).unwrap_or_else(||"Подписка".into());let json_config=serde_json::from_str::<Value>(&decoded).ok().filter(|value|value["outbounds"].as_array().is_some()).or_else(||serde_json::from_str::<Value>(body.trim()).ok().filter(|value|value["outbounds"].as_array().is_some()));let title=header_name.filter(|v|!v.trim().is_empty()).or_else(||decoded.lines().next().and_then(|line|line.trim().strip_prefix('#')).map(str::trim).filter(|line|!line.is_empty()).map(str::to_owned));let name=title.map(|value|profile_title(&value,&fallback_name)).unwrap_or_else(||json_config.as_ref().and_then(|value|value["name"].as_str()).filter(|name|!name.trim().is_empty()).unwrap_or(&fallback_name).to_owned());let description=announce.as_deref().map(announce_description).filter(|value|!value.is_empty()).or_else(||decoded.lines().filter_map(|line|line.trim().strip_prefix('#').map(str::trim)).find(|line|!line.is_empty()&&!line.to_ascii_lowercase().contains("upload=")&&!line.to_ascii_lowercase().contains("download=")&&!line.to_ascii_lowercase().contains("expire=")).map(str::to_owned)).or_else(||json_config.as_ref().and_then(|value|value["description"].as_str()).map(str::to_owned)).unwrap_or_default();
let subscription_id=s.subscriptions.iter().find(|existing|existing.source==input).map(|existing|existing.id.clone()).unwrap_or_else(uid);let mut sub=Subscription{id:subscription_id,name,source:input.into(),upload:0,download:0,total:0,expire:0,description};s.servers.retain(|server|server.group!=sub.id);for part in info.split(';'){if let Some((k,v))=part.trim().split_once('='){let n=v.parse().unwrap_or(0);match k.to_ascii_lowercase().as_str(){"upload"=>sub.upload=n,"download"=>sub.download=n,"total"=>sub.total=n,"expire"=>sub.expire=n,_=>()}}}let mut imported=subscription_servers(&decoded,&sub.id);if imported.is_empty(){let direct_body=subscription_servers(body.trim(),&sub.id);if !direct_body.is_empty(){imported=direct_body;}}if imported.is_empty(){if let Some(outbounds)=json_config.as_ref().and_then(|value|value["outbounds"].as_array()){imported=outbounds.iter().filter_map(|outbound|singbox_server(outbound,&sub.id)).collect();}}let mut count=0;for mut server in imported{server.id=format!("{}_{}",sub.id,count);server.group=sub.id.clone();server.source=Some(input.into());s.servers.push(server);count+=1;}let _=app.emit("engine-log",format!("VPN subscription response: {response_len} bytes, {count} servers parsed and stored for {}",sub.id));if let Some(existing)=s.subscriptions.iter_mut().find(|existing|existing.source==input){let old_id=existing.id.clone();*existing=sub.clone();if old_id!=sub.id{for server in &mut s.servers{if server.group==old_id{server.group=sub.id.clone();server.id=server.id.replacen(&old_id,&sub.id,1);}}}}else{s.subscriptions.push(sub);}save(&app,&s)?;Ok(json!({"count":count,"state":s}))}else{let server=parse_link(input,"manual")?;s.servers.push(server);save(&app,&s)?;Ok(json!({"count":1,"state":s}))}}
use sha2::Digest;
#[tauri::command]
pub async fn update_vpn_subscription(app:AppHandle,sub_id:String)->Result<Value,String>{let mut s=load(&app)?;let sub=s.subscriptions.iter().find(|x|x.id==sub_id).ok_or("Подписка не найдена")?;let source=sub.source.clone();s.servers.retain(|server|server.group!=sub_id);s.subscriptions.retain(|item|item.id!=sub_id);save(&app,&s)?;import_vpn_source(app,source).await}
#[tauri::command]
pub fn delete_vpn_item(app:AppHandle,id:String)->Result<Value,String>{let mut s=load(&app)?;let is_subscription=s.subscriptions.iter().any(|x|x.id==id);if is_subscription{s.servers.retain(|server|server.group!=id);s.subscriptions.retain(|x|x.id!=id);}else{s.servers.retain(|server|server.id!=id);}if s.selected_id.as_deref()==Some(&id)||!s.servers.iter().any(|server|Some(server.id.as_str())==s.selected_id.as_deref()){s.selected_id=None}save(&app,&s)?;Ok(json!(s))}
#[tauri::command]
pub fn get_vpn_state(app:AppHandle)->Result<State,String>{load(&app)}
#[tauri::command]
pub fn select_vpn_server(app:AppHandle,id:String)->Result<(),String>{let mut s=load(&app)?;if !s.servers.iter().any(|x|x.id==id){return Err("Сервер не найден".into())}s.selected_id=Some(id);save(&app,&s)}
#[tauri::command]
pub async fn ping_vpn_servers(app:AppHandle)->Result<Vec<Server>,String>{let mut s=load(&app)?;let servers=s.servers.clone();let results=tokio::task::spawn_blocking(move||{let mut handles=Vec::new();for server in servers{handles.push(std::thread::spawn(move||{let addr=format!("{}:{}",server.address,server.port).to_socket_addrs().ok().and_then(|mut i|i.next());let ping=addr.and_then(|a:SocketAddr|{let now=Instant::now();TcpStream::connect_timeout(&a,Duration::from_secs(3)).ok().map(|_|now.elapsed().as_millis() as i64)}).unwrap_or(-1);(server.id,ping)}));}handles.into_iter().filter_map(|h|h.join().ok()).collect::<Vec<_>>()}).await.map_err(|e|e.to_string())?;for server in &mut s.servers{if let Some((_,p))=results.iter().find(|(id,_)|id==&server.id){server.ping=*p}}save(&app,&s)?;Ok(s.servers)}
fn process_running(name: &str) -> bool {
    let mut command = Command::new("tasklist.exe");
    command.args(["/FI", &format!("IMAGENAME eq {name}"), "/FO", "CSV", "/NH"])
        .stdin(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command.output().ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .is_some_and(|output| output.lines().any(|line| {
            line.trim_start_matches('"').split('"').next()
                .is_some_and(|image| image.eq_ignore_ascii_case(name))
        }))
}

#[tauri::command]
pub async fn check_network_diagnostics() -> Result<Vec<Value>, String> {
    let mut results = tokio::task::spawn_blocking(|| {
        [("https://cloudflare.com", "Cloudflare"), ("https://api.telegram.org", "Telegram API"), ("https://youtube.com", "YouTube"), ("https://discord.com/api/v10/gateway", "Discord")]
            .into_iter()
            .map(|(target, label)| { let (ok, latency_ms) = diagnostic_probe(target); json!({"target": target, "label": label, "ok": ok, "latencyMs": latency_ms}) })
            .collect::<Vec<_>>()
    }).await.map_err(|error| error.to_string())?;
    for name in ["winws.exe", "ciadpi.exe", "tg-ws-proxy.exe", "sing-box.exe"] {
        let running = tokio::task::spawn_blocking(move || process_running(name)).await.map_err(|error| error.to_string())?;
        results.push(json!({"target": format!("process:{name}"), "label": name, "ok": running, "latencyMs": 0}));
    }
    Ok(results)
}

#[cfg(windows)]
fn set_system_proxy(enabled: bool, port: u16) -> Result<(), String> {
    const INTERNET_OPTION_SETTINGS_CHANGED: u32 = 39;
    const INTERNET_OPTION_REFRESH: u32 = 37;
    #[link(name = "wininet")]
    unsafe extern "system" { fn InternetSetOptionW(handle: *mut std::ffi::c_void, option: u32, buffer: *mut std::ffi::c_void, length: u32) -> i32; }
    let key = "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings";
    let mut set_value = |name: &str, kind: &str, value: Option<String>| -> Result<(), String> {
        let mut command = Command::new("reg.exe");
        let required = value.is_some();
        if let Some(value) = value {
            command.args(["add", key, "/v", name, "/t", kind, "/d"]).arg(value).args(["/f"]);
        } else {
            command.args(["delete", key, "/v", name, "/f"]);
        }
        command.creation_flags(0x08000000);
        let output = command.output().map_err(|error| error.to_string())?;
        if !output.status.success() && required { return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned()); }
        Ok(())
    };
    if enabled {
        set_value("ProxyServer", "REG_SZ", Some(format!("127.0.0.1:{port}")))?;
        set_value("ProxyOverride", "REG_SZ", Some("<local>".into()))?;
        set_value("ProxyEnable", "REG_DWORD", Some("1".into()))?;
    } else {
        set_value("ProxyEnable", "REG_DWORD", Some("0".into()))?;
        set_value("ProxyServer", "REG_SZ", None)?;
        set_value("ProxyOverride", "REG_SZ", None)?;
    }
    unsafe {
        InternetSetOptionW(std::ptr::null_mut(), INTERNET_OPTION_SETTINGS_CHANGED, std::ptr::null_mut(), 0);
        InternetSetOptionW(std::ptr::null_mut(), INTERNET_OPTION_REFRESH, std::ptr::null_mut(), 0);
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn reset_system_proxy() -> Result<(), String> { set_system_proxy(false, 0) }

#[cfg(not(windows))]
pub(crate) fn reset_system_proxy() -> Result<(), String> { Ok(()) }

#[cfg(not(windows))]
fn set_system_proxy(_enabled: bool, _port: u16) -> Result<(), String> { Ok(()) }

#[tauri::command]
pub async fn connect_vpn(app:AppHandle,id:Option<String>,routing:Option<String>)->Result<(),String>{
  #[cfg(windows)]
  {
   let _=Command::new("taskkill").args(["/F","/IM","sing-box.exe","/T"]).creation_flags(0x08000000).output();
  }
  tokio::time::sleep(Duration::from_millis(300)).await;
 let prefs=crate::settings::settings(&app);let mut s=load(&app)?;let id=id.or(s.selected_id.clone()).ok_or("Сначала выберите VPN-сервер")?;let server=s.servers.iter().find(|x|x.id==id).ok_or("VPN-сервер не найден")?.clone();s.selected_id=Some(id.clone());save(&app,&s)?;let exe=crate::downloader::ensure_binary(&app,"sing-box.exe").await?;let dir=app.path().app_local_data_dir().map_err(|e|e.to_string())?.join("vpn");fs::create_dir_all(&dir).map_err(|e|e.to_string())?;let mut route_mode=if routing.as_deref()==Some("split"){"system"}else{routing.as_deref().unwrap_or(prefs["routing"].as_str().unwrap_or("system"))}; if !matches!(route_mode,"system"|"direct_apps"|"vpn_apps"){route_mode="system";} let split_tunnel=prefs["split_tunneling"].as_bool().or_else(||prefs["splitTunneling"].as_bool()).unwrap_or(false); let app_rules=prefs["apps"].as_array().cloned().unwrap_or_default(); let app_rule=if route_mode=="direct_apps"&&!app_rules.is_empty(){vec![json!({"process_name":app_rules,"outbound":"direct"})]}else{Vec::<Value>::new()}; let dns_rules=if split_tunnel {json!([{"domain_suffix":[".ru",".рф",".su"],"server":"local"}])}else{json!([])}; let rule_set=json!([]); let mut route_rules=vec![json!({"protocol":"dns","action":"hijack-dns"}),json!({"ip_is_private":true,"outbound":"direct"})]; if split_tunnel { route_rules.push(json!({"domain_suffix":[".ru",".рф",".su"],"outbound":"direct"})); } if route_mode=="vpn_apps"&&!app_rules.is_empty(){route_rules.push(json!({"process_name":app_rules,"outbound":"proxy"}));}else{route_rules.extend(app_rule);} let lan=prefs["lan"].as_bool().unwrap_or(false);
 let requested_proxy=prefs["mode"].as_str()==Some("proxy"); if !requested_proxy { crate::downloader::ensure_wintun(&exe).await?; }
 if let Ok(mut guard)=child().lock(){if let Some(mut previous)=guard.take(){let _=previous.kill();let _=previous.wait();}} let proxy_mode=requested_proxy; let listen=if lan{"0.0.0.0"}else{"127.0.0.1"}; let http=prefs["httpPort"].as_u64().and_then(|port|u16::try_from(port).ok()).unwrap_or(2081); let use_system_proxy=requested_proxy; if use_system_proxy { set_system_proxy(true,10808)?; } else { set_system_proxy(false,0)?; } let mut inbounds=if proxy_mode { vec![json!({"type":"mixed","tag":"mixed-in","listen":"127.0.0.1","listen_port":10808})] } else { vec![json!({"type":"socks","tag":"socks-in","listen":listen,"listen_port":prefs["socksPort"].as_u64().unwrap_or(2080),"users":[]}),json!({"type":"http","tag":"http-in","listen":listen,"listen_port":http}),json!({"type":"tun","tag":"tun-in","interface_name":"AltFy-TUN","address":["172.19.0.1/30"],"mtu":1400,"auto_route":true,"strict_route":false,"stack":"gvisor"})] }; let mut outbound=server.config.clone(); if let Some(outbound_fields)=outbound.as_object_mut(){outbound_fields.remove("domain_strategy");} if matches!(server.protocol.to_ascii_lowercase().as_str(),"vless"|"trojan"|"vmess"|"hysteria2"){if let Some(tls)=outbound.get_mut("tls").filter(|value|value.is_object()){tls["utls"]=json!({"enabled":true,"fingerprint":"chrome"});}} if prefs["mux"].as_bool().unwrap_or(false){outbound["multiplex"]=json!({"enabled":true,"protocol":"h2mux","max_connections":4,"min_streams":4,"max_streams":0});} if prefs["fragmentation"].as_bool().unwrap_or(false){let size=prefs["packetLength"].as_str().filter(|value|!value.is_empty()).map(|value|format!("{}-{}",value,value)).unwrap_or_else(||"1-500".into());let sleep=prefs["packetInterval"].as_str().filter(|value|!value.is_empty()).map(|value|format!("{}-{}",value,value)).unwrap_or_else(||"10-20".into());if let Some(tls)=outbound.get_mut("tls").filter(|value|value.is_object()){tls["fragment"]=json!({"enabled":true,"size":size,"sleep":sleep});}} let final_route=if route_mode=="vpn_apps"{"direct"}else{"proxy"}; let config=json!({"log":{"level":"info","timestamp":true},"dns":{"servers":[{"tag":"remote","type":"udp","server":"1.1.1.1","detour":"proxy"},{"tag":"local","type":"udp","server":"223.5.5.5"}],"rules":dns_rules,"final":"remote"},"inbounds":inbounds,"outbounds":[outbound,{"type":"direct","tag":"direct"}],"route":{"auto_detect_interface":true,"default_domain_resolver":"remote","rule_set":rule_set,"rules":route_rules,"default_domain_resolver":"local","final":if route_mode=="vpn_apps"{"direct"}else{"proxy"}}});let cfg=dir.join("config.json");fs::write(&cfg,serde_json::to_vec_pretty(&config).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;if proxy_mode {#[cfg(windows)]{let _=Command::new("reg").args(["add","HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings","/v","ProxyServer","/t","REG_SZ","/d",&"127.0.0.1:10808".to_owned(),"/f"]).output();let _=Command::new("reg").args(["add","HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings","/v","ProxyEnable","/t","REG_DWORD","/d","1","/f"]).output();}} let mut lock=child().lock().map_err(|e|e.to_string())?;if let Some(mut old)=lock.take(){let _=old.kill();}let mut cmd=Command::new(exe);cmd.args(["run","-c"]).arg(cfg).stdout(Stdio::piped()).stderr(Stdio::piped());#[cfg(windows)]{use std::os::windows::process::CommandExt;cmd.creation_flags(0x08000000);}let mut proc=cmd.spawn().map_err(|e|e.to_string())?;if let Some(reader)=proc.stdout.take(){let handle=app.clone();std::thread::spawn(move||{use std::io::{BufRead,BufReader};for line in BufReader::new(reader).lines().flatten(){let _=handle.emit("engine-log",json!({"engine":"sing-box","stream":"stdout","message":line}));}});}if let Some(reader)=proc.stderr.take(){let handle=app.clone();std::thread::spawn(move||{use std::io::{BufRead,BufReader};for line in BufReader::new(reader).lines().flatten(){let _=handle.emit("engine-log",json!({"engine":"sing-box","stream":"stderr","message":line}));}});}*lock=Some(proc);s.selected_id=Some(id);save(&app,&s)}
#[tauri::command]
pub fn get_vpn_status()->bool{child().lock().ok().and_then(|mut guard|guard.as_mut().map(|process|match process.try_wait(){Ok(Some(_))=>false,Ok(None)=>true,Err(_)=>false})).unwrap_or(false)}
#[tauri::command]
pub fn disconnect_vpn()->Result<(),String>{#[cfg(windows)] use std::os::windows::process::CommandExt;if let Ok(mut c)=child().lock(){if let Some(mut p)=c.take(){let _=p.kill();let _=p.wait();}} #[cfg(windows)]{let _=Command::new("taskkill").args(["/F","/IM","sing-box.exe","/T"]).creation_flags(0x08000000).output();} reset_system_proxy()}
