const S={plugins:[],sel:null,action:null,loading:false,ws:null,dash:null,theme:localStorage.getItem('theme')||'dark',pollTimer:null,prevDash:{}};
const $=id=>document.getElementById(id);
const esc=s=>String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
const fmtB=b=>{if(!b||b<0)return'0B';if(b<1024)return b+'B';if(b<1048576)return(b/1024).toFixed(1)+'K';if(b<1073741824)return(b/1048576).toFixed(1)+'M';return(b/1073741824).toFixed(2)+'G'};
const fmtUp=s=>{const d=Math.floor(s/86400),h=Math.floor(s%86400/3600),m=Math.floor(s%3600/60);return d>0?d+'d '+h+'h '+m+'m':h>0?h+'h '+m+'m':m+'m'};
function toast(m,t){const c=$('toasts'),e=document.createElement('div');e.className='toast '+(t||'');e.textContent=m;c.appendChild(e);setTimeout(()=>{e.style.opacity='0';e.style.transition='opacity 0.3s';setTimeout(()=>e.remove(),300)},2500)}
async function api(p,o={}){try{const r=await fetch(p,{headers:{'Content-Type':'application/json'},...o});const d=await r.json();if(!r.ok)throw new Error(d.error||'HTTP '+r.status);return d}catch(e){throw e}}
function showModal(t,h,onSave){$('modal-title').textContent=t;$('modal-body').innerHTML=h;$('modal-bg').style.display='flex';$('modal-save').onclick=onSave}
function closeModal(){$('modal-bg').style.display='none'}
window.closeModal=closeModal;
function flash(id,v){const el=$(id);if(!el)return;const o=el.getAttribute('data-v');if(o!==String(v)){el.setAttribute('data-v',String(v));el.classList.remove('data-flash');void el.offsetWidth;el.classList.add('data-flash')}}
function setT(id,v){const el=$(id);if(el)el.textContent=v}

// Theme
function setTheme(t){S.theme=t;document.documentElement.setAttribute('data-theme',t);localStorage.setItem('theme',t)}
setTheme(S.theme);
$('theme-toggle').onclick=()=>setTheme(S.theme==='dark'?'light':'dark');

// Clock
setInterval(()=>{const el=$('clock');if(el)el.textContent=new Date().toLocaleTimeString();const ft=$('footer-time');if(ft)ft.textContent=new Date().toLocaleTimeString()},1000);

// Particles
(function(){const c=$('particles'),x=c.getContext('2d');let w,h,pts=[];function rs(){w=c.width=window.innerWidth;h=c.height=window.innerHeight}rs();window.addEventListener('resize',rs);
for(let i=0;i<30;i++)pts.push({x:Math.random()*w,y:Math.random()*h,vx:(Math.random()-.5)*.15,vy:(Math.random()-.5)*.15,r:Math.random()*.8+.2});
(function dr(){x.clearRect(0,0,w,h);for(let p of pts){p.x+=p.vx;p.y+=p.vy;if(p.x<0||p.x>w)p.vx*=-1;if(p.y<0||p.y>h)p.vy*=-1;x.beginPath();x.arc(p.x,p.y,p.r,0,Math.PI*2);x.fillStyle='rgba(34,211,238,0.03)';x.fill()}
for(let i=0;i<pts.length;i++)for(let j=i+1;j<pts.length;j++){const d=Math.hypot(pts[i].x-pts[j].x,pts[i].y-pts[j].y);if(d<80){x.beginPath();x.moveTo(pts[i].x,pts[i].y);x.lineTo(pts[j].x,pts[j].y);x.strokeStyle=`rgba(34,211,238,${.01*(1-d/80)})`;x.stroke()}}
requestAnimationFrame(dr)})();})();

// Tabs
document.querySelectorAll('.nav-btn').forEach(t=>{t.onclick=()=>{document.querySelectorAll('.nav-btn').forEach(x=>x.classList.remove('active'));document.querySelectorAll('.panel').forEach(x=>x.classList.remove('active'));t.classList.add('active');const v=$('panel-'+t.dataset.tab);if(v)v.classList.add('active');
switch(t.dataset.tab){case'dashboard':startDash();break;case'plugins':loadPlugins();break;case'docker':D.refresh();break;case'middleware':MW.init();break}}});
document.querySelectorAll('.dtab').forEach(t=>{t.onclick=()=>{document.querySelectorAll('.dtab').forEach(x=>x.classList.remove('active'));document.querySelectorAll('.dtp').forEach(x=>x.classList.remove('active'));t.classList.add('active');const p=$('dt-'+t.dataset.dt);if(p)p.classList.add('active');
if(t.dataset.dt==='compose')D.loadCompose();if(t.dataset.dt==='containers')D.refresh();if(t.dataset.dt==='images')D.loadImages()}});

// Dashboard
function startDash(){
  // Always poll as baseline
  if(!S.pollTimer){async function poll(){try{const d=await api('/api/dashboard');updateDash(d)}catch(e){}}poll();S.pollTimer=setInterval(poll,2000)}
  // Try WS upgrade
  if(S.ws)return;
  try{
    const ws=new WebSocket((location.protocol==='https:'?'wss:':'ws:')+'//'+location.host+'/ws/dashboard');
    ws.onopen=()=>{$('live-dot').style.background='var(--green)';$('live-label').textContent='实时';if(S.pollTimer){clearInterval(S.pollTimer);S.pollTimer=null}};
    ws.onmessage=e=>{try{updateDash(JSON.parse(e.data))}catch(err){}};
    ws.onerror=()=>{};
    ws.onclose=()=>{$('live-dot').style.background='var(--amber)';$('live-label').textContent='轮询';if(!S.pollTimer){async function poll(){try{const d=await api('/api/dashboard');updateDash(d)}catch(e){}}poll();S.pollTimer=setInterval(poll,2000)};S.ws=null;setTimeout(startDash,5000)};
    S.ws=ws;
  }catch(e){$('live-dot').style.background='var(--amber)';$('live-label').textContent='轮询'}
}

function updateDash(d){S.dash=d;
setT('d-hostname',d.hostname||'-');setT('d-arch',d.arch||'-');

// CPU
const cp=Math.min(100,Math.max(0,d.cpu_pct||0));
$('cpu-ring').style.strokeDashoffset=213.6*(1-cp/100);
flash('cpu-pct',Math.round(cp));setT('cpu-pct',Math.round(cp));
const cb=$('cpu-badge');if(cb){cb.textContent=Math.round(cp)+'%';cb.className='mc-badge '+(cp>90?'mc-badge-err':cp>70?'mc-badge-warn':'')}
flash('d-load1',(d.load1||0).toFixed(2));setT('d-load1',(d.load1||0).toFixed(2));
flash('d-load5',(d.load5||0).toFixed(2));setT('d-load5',(d.load5||0).toFixed(2));
flash('d-load15',(d.load15||0).toFixed(2));setT('d-load15',(d.load15||0).toFixed(2));

// Core bars
const cores=d.cpu_cores_pct||[];
const cs=$('core-strip');
if(cs&&cores.length){let bars=cs.children;if(bars.length!==cores.length){cs.innerHTML='';cores.forEach(()=>{const b=document.createElement('div');b.className='core-bar';cs.appendChild(b)});bars=cs.children}
cores.forEach((c,i)=>{if(bars[i])bars[i].style.height=Math.max(1,Math.min(100,c)*0.18)+'px'})}

// Memory
const mp=d.mem_total>0?Math.round(d.mem_used/d.mem_total*100):0;
$('mem-ring').style.strokeDashoffset=213.6*(1-mp/100);
flash('mem-pct',mp);setT('mem-pct',mp);
const mb=$('mem-badge');if(mb){mb.textContent=mp+'%';mb.className='mc-badge '+(mp>90?'mc-badge-err':mp>80?'mc-badge-warn':'')}
const mf=$('mem-fill');if(mf)mf.style.width=mp+'%';
if(d.swap_total>0){const sp=Math.round(d.swap_used/d.swap_total*100);const sf=$('swap-fill');if(sf)sf.style.width=sp+'%'}else{const sf=$('swap-fill');if(sf)sf.style.width='0%'}
flash('mem-used',fmtB(d.mem_used*1024));setT('mem-used',fmtB(d.mem_used*1024));
flash('mem-total',fmtB(d.mem_total*1024));setT('mem-total',fmtB(d.mem_total*1024));
flash('mem-cached',fmtB(d.mem_cached*1024));setT('mem-cached',fmtB(d.mem_cached*1024));
setT('mem-swap',fmtB(d.swap_used*1024)+'/'+fmtB(d.swap_total*1024));

// Disks
const db=$('disk-body');
if(db){let h='';(d.disks||[]).forEach(i=>{const p=parseInt(i.pct)||0;const c=p>90?'var(--rose)':p>80?'var(--amber)':'var(--cyan)';h+=`<div class="disk-row"><span class="disk-dev">${esc(i.dev)}</span><span class="disk-mount">${esc(i.mount)}</span><div class="disk-bar"><div class="disk-fill" style="width:${p}%;background:${c}"></div></div><span class="disk-info">${esc(i.used)}/${esc(i.size)} ${esc(i.pct)}</span></div>`});db.innerHTML=h||'<div class="mc-empty">无磁盘</div>'}
const dib=$('disk-badge');if(dib){const root=(d.disks||[]).find(x=>x.mount==='/');dib.textContent=root?root.pct:'—'}
flash('disk-r',fmtB(d.disk_read_kb*1024));setT('disk-r',fmtB(d.disk_read_kb*1024));
flash('disk-w',fmtB(d.disk_write_kb*1024));setT('disk-w',fmtB(d.disk_write_kb*1024));

// Network
const nb=$('net-body');
if(nb){let h='<table><tr><th>接口</th><th>RX</th><th>TX</th></tr>';(d.ifaces||[]).forEach(n=>{h+=`<tr><td>${esc(n.name)}</td><td>${fmtB(n.rx_bytes)}</td><td>${fmtB(n.tx_bytes)}</td></tr>`});h+='</table>';nb.innerHTML=h||'<div class="mc-empty">无网络</div>'}
const nib=$('net-badge');if(nib){nib.textContent=(d.ifaces||[]).length+' 接口'}
const ni=$('net-ips');if(ni)ni.textContent=d.ips&&d.ips.length?'IP: '+d.ips.join(', '):'—';

// Ports
const pl=$('port-table');
if(pl){let h='<table><tr><th>端口</th><th>协议</th><th>进程</th><th>PID</th></tr>';(d.ports||[]).slice(0,15).forEach(p=>{h+=`<tr><td style="color:var(--cyan);font-weight:600">${p.port}</td><td>${p.proto}</td><td>${esc(p.process||'-')}</td><td>${p.pid||'-'}</td></tr>`});h+='</table>';pl.innerHTML=(d.ports&&d.ports.length)?h:'<div class="mc-empty">无端口</div>'}
const pc=$('port-count');if(pc)pc.textContent=(d.ports||[]).length;

// Top CPU
const tc=$('cpu-top-table');
if(tc){let h='<table><tr><th>进程</th><th>PID</th><th>CPU</th><th>MEM</th></tr>';(d.top_cpu||[]).slice(0,8).forEach(p=>{h+=`<tr><td>${esc(p.name)}</td><td>${p.pid}</td><td style="color:${p.cpu>50?'var(--rose)':p.cpu>20?'var(--amber)':'var(--text2)'}">${p.cpu.toFixed(1)}</td><td>${fmtB(p.mem_kb*1024)}</td></tr>`});h+='</table>';tc.innerHTML=h}

// Top Mem
const tm=$('mem-top-table');
if(tm){let h='<table><tr><th>进程</th><th>PID</th><th>MEM</th></tr>';(d.top_mem||[]).slice(0,8).forEach(p=>{h+=`<tr><td>${esc(p.name)}</td><td>${p.pid}</td><td>${fmtB(p.mem_kb*1024)}</td></tr>`});h+='</table>';tm.innerHTML=h}

// Apps
const al=$('app-table');
if(al){let h='<table><tr><th>应用</th><th>服务</th><th>PID</th><th>类型</th><th>端口</th><th>路径</th><th>线程</th></tr>';
(d.apps||[]).forEach(a=>{h+=`<tr><td style="font-weight:500">${esc(a.name.substring(0,30))}</td><td style="color:var(--cyan)">${esc(a.service_name||'-')}</td><td>${a.pid}</td><td><span style="color:var(--primary)">${esc(a.category)}</span></td><td style="color:var(--green)">${a.ports.join(',')||'-'}</td><td style="font-size:9px;color:var(--text3);max-width:100px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap" title="${esc(a.exe_path)}">${esc(a.exe_path||'-')}</td><td>${a.threads||'-'}</td></tr>`});
h+='</table>';al.innerHTML=(d.apps&&d.apps.length)?h:'<div class="mc-empty">无应用</div>'}
const ac=$('app-count');if(ac)ac.textContent=(d.apps||[]).length;

// Errors
const et=$('err-table');
if(et){const errs=d.sys_errors||[];if(errs.length){let h='<table><tr><th>服务</th><th>消息</th><th>级别</th><th>时间</th></tr>';
errs.forEach(e=>{const c=e.severity==='critical'?'var(--rose)':e.severity==='error'?'var(--amber)':'var(--text3)';h+=`<tr><td style="font-weight:500">${esc(e.service)}</td><td style="max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc(e.message)}</td><td style="color:${c}">${esc(e.severity)}</td><td style="font-size:9px">${esc(e.timestamp)}</td></tr>`});
h+='</table>';et.innerHTML=h}else{et.innerHTML='<div class="mc-ok">✓ 无最近错误</div>'}}
const ec=$('err-count');if(ec)ec.textContent=(d.sys_errors||[]).length;

// Anomalies
const an=d.anomalies||[];
const ab=$('alert-badge'),ab2=$('alert-body');
if(ab){if(an.length){ab.textContent=an.length+' 告警';ab.className='mc-badge mc-badge-err'}else{ab.textContent='正常';ab.className='mc-badge mc-badge-ok'}}
if(ab2){if(an.length){ab2.innerHTML=an.map(a=>`<div style="padding:2px 0;font-size:10px;color:var(--amber);border-bottom:1px solid var(--border2)">⚠ ${esc(a)}</div>`).join('')}else{ab2.innerHTML='<div class="mc-ok">✓ 系统正常</div>'}}

// Footer
const fi=$('footer-info');if(fi)fi.textContent=`进程:${d.procs||0} 线程:${d.threads||0} FD:${d.fd_cur||0}/${d.fd_max||0} 僵尸:${d.zombies||0} HW:${d.hw_vendor||''} ${d.hw_model||''}`;
}

// Plugins
async function loadPlugins(){try{S.plugins=(await api('/api/plugins')).plugins||[];renderPL();if(S.plugins.length&&!S.sel)selP(S.plugins[0].name)}catch(e){}}
function renderPL(){const q=($('p-search')?.value||'').toLowerCase();$('p-list').innerHTML=S.plugins.filter(p=>!q||p.name.toLowerCase().includes(q)||p.category.toLowerCase().includes(q)).map(p=>`<div class="p-item${S.sel===p.name?' active':''}" onclick="selP('${esc(p.name)}')"><div class="p-icon ${piCls(p.category)}">${piCh(p.category)}</div><div class="p-info"><div class="p-name">${esc(p.name)}</div><div class="p-meta">${esc(p.category)} · v${esc(p.version)}</div></div></div>`).join('')||'<div class="mc-empty">无插件</div>'}
function selP(n){S.sel=n;S.action=null;const p=S.plugins.find(x=>x.name===n);if(!p)return;
$('p-name').textContent=p.name;$('p-cat').textContent=p.category;$('p-desc').textContent=p.description;$('p-meta').innerHTML=`<span>v${esc(p.version)}</span><span>${p.actions.length} 操作</span>`;
$('act-grid').innerHTML=p.actions.length?p.actions.map(a=>`<button class="act-btn${S.action===a.name?' active':''}" onclick="selA('${esc(a.name)}')">${esc(a.name)}</button>`).join(''):'<div class="mc-empty">无操作</div>';
if(!S.action&&p.actions.length)selA(p.actions[0].name);renderPL()}
window.selP=selP;
function selA(n){S.action=n;document.querySelectorAll('.act-btn').forEach(b=>b.classList.toggle('active',b.textContent===n))}
window.selA=selA;
async function runP(){if(!S.sel||!S.action)return toast('选择插件和操作','error');if(S.loading)return;S.loading=true;
const btn=$('btn-run'),out=$('p-output'),ph=$('out-ph');
btn.disabled=true;btn.textContent='运行中...';ph.classList.add('hidden');out.textContent='';
try{const d=await api('/api/plugins/'+S.sel+'/execute',{method:'POST',body:JSON.stringify({action:S.action,input_data:$('p-input').value||null})});
out.textContent=d.data||d.error||'(空)';out.style.color=d.success?'var(--text)':'var(--rose)';toast(d.success?'完成':'失败',d.success?'success':'error')}catch(e){out.textContent=e.message;out.style.color='var(--rose)'}
finally{S.loading=false;btn.disabled=false;btn.textContent='▶ 运行'}}
const uz=$('upload-zone'),fi=$('file-input');
if(uz){uz.onclick=()=>fi.click();uz.ondragover=e=>{e.preventDefault();uz.style.borderColor='var(--cyan)'};uz.ondragleave=()=>{uz.style.borderColor='var(--border)'};uz.ondrop=e=>{e.preventDefault();uz.style.borderColor='var(--border)';[...e.dataTransfer.files].forEach(upF)};fi.onchange=()=>{[...fi.files].forEach(upF);fi.value=''}}
async function upF(f){if(!f.name.match(/\.(so|dylib|dll)$/))return toast('仅 .so/.dylib/.dll','error');toast('上传 '+f.name+'...');try{const b=await f.arrayBuffer(),r=await fetch('/api/plugins/upload?name='+encodeURIComponent(f.name),{method:'POST',body:b}),d=await r.json();toast(d.status==='uploaded'?d.name+' 已加载':d.warning||'已保存',d.status==='uploaded'?'success':'error');await loadPlugins()}catch(e){toast(e.message,'error')}}
if($('btn-run'))$('btn-run').onclick=runP;
if($('btn-clear'))$('btn-clear').onclick=()=>{$('p-input').value='';$('p-output').textContent='';$('out-ph').classList.remove('hidden')};
if($('p-search'))$('p-search').oninput=renderPL;
document.onkeydown=e=>{if(e.ctrlKey&&e.key==='Enter'){e.preventDefault();runP()}};
function piCls(c){return'pi-'+({DataTool:'d',SystemTool:'s',Security:'k',Middleware:'m',Script:'r',Network:'n'}[c]||'d')}
function piCh(c){return{DataTool:'D',SystemTool:'S',Security:'K',Middleware:'M',Script:'R',Network:'N'}[c]||'?'}

// Docker
const D={containers:[],async refresh(){try{const c=await api('/api/docker/containers');D.containers=c.containers||[];
let h='<table><tr><th>ID</th><th>名称</th><th>镜像</th><th>状态</th><th>CPU</th><th>MEM</th><th>操作</th></tr>';
D.containers.forEach(c=>{const r=c.status.includes('Up');h+=`<tr><td style="font-family:monospace;font-size:9px">${esc(c.id.substring(0,12))}</td><td style="font-weight:500">${esc(c.name)}</td><td>${esc(c.image)}</td><td class="${r?'text-ok':'text-err'}">${r?'●':'○'} ${esc(c.status)}</td><td>${esc(c.cpu)}</td><td>${esc(c.mem)}</td><td class="docker-acts"><button class="btn btn-sm btn-ghost" onclick="D.act('${c.id}','${r?'stop':'start'}')">${r?'停':'启'}</button><button class="btn btn-sm btn-ghost" onclick="D.act('${c.id}','restart')">重启</button><button class="btn btn-sm btn-ghost" onclick="D.logs('${c.id}')">日志</button><button class="btn btn-sm btn-ghost" onclick="D.inspect('${c.id}')">详情</button></td></tr>`});
h+='</table>';$('d-containers').innerHTML=D.containers.length?h:'<div class="mc-empty">无容器</div>'}catch(e){$('d-containers').innerHTML='<div class="mc-empty" style="color:var(--rose)">Docker 不可用</div>'}},
async loadImages(){try{const d=await api('/api/docker/images');let h='<table><tr><th>仓库</th><th>标签</th><th>ID</th><th>大小</th></tr>';
(d.images||[]).forEach(i=>{h+=`<tr><td>${esc(i.repo)}</td><td>${esc(i.tag)}</td><td style="font-family:monospace;font-size:9px">${esc(i.id.substring(0,12))}</td><td>${esc(i.size)}</td></tr>`});h+='</table>';$('d-images').innerHTML=h||'<div class="mc-empty">无镜像</div>'}catch(e){}},
async loadCompose(){try{const d=await api('/api/docker/compose');const ps=d.projects||[];
if(!ps.length){$('d-compose').innerHTML='<div class="mc-empty">无 Compose 项目</div>';return}
let h='';ps.forEach(p=>{h+=`<div style="margin-bottom:8px;border:1px solid var(--border);border-radius:6px;overflow:hidden"><div style="padding:6px 10px;background:var(--bg3);display:flex;align-items:center;gap:6px"><span style="font-weight:600;font-size:12px">${esc(p.name)}</span><span class="mc-badge">${esc(p.status)}</span><span style="flex:1"></span><button class="btn btn-sm btn-ghost" onclick="D.composeAct('${esc(p.name)}','up')">启动</button><button class="btn btn-sm btn-ghost" onclick="D.composeAct('${esc(p.name)}','down')">停止</button><button class="btn btn-sm btn-ghost" onclick="D.composeAct('${esc(p.name)}','restart')">重启</button><button class="btn btn-sm btn-ghost" onclick="D.composeLogs('${esc(p.name)}')">日志</button></div><table><tr><th>服务</th><th>镜像</th><th>状态</th><th>端口</th></tr>`;
p.services.forEach(s=>{const r=s.status.includes('Up');h+=`<tr><td style="font-weight:500">${esc(s.name)}</td><td>${esc(s.image)}</td><td class="${r?'text-ok':'text-err'}">${r?'●':'○'} ${esc(s.status)}</td><td>${esc(s.ports||'-')}</td></tr>`});h+='</table></div>'});$('d-compose').innerHTML=h}catch(e){$('d-compose').innerHTML='<div class="mc-empty" style="color:var(--rose)">加载失败</div>'}},
async act(id,a){try{await api('/api/docker/containers/'+id+'/'+a,{method:'POST'});toast(a+' 成功','success');setTimeout(()=>D.refresh(),800)}catch(e){toast(a+' 失败','error')}},
async logs(id){try{const d=await api('/api/docker/containers/'+id+'/logs');$('d-output').textContent=d.logs||'(空)'}catch(e){$('d-output').textContent='错误: '+e.message}},
async inspect(id){try{const d=await api('/api/docker/containers/'+id+'/inspect');$('d-output').textContent=d.inspect||'(空)'}catch(e){$('d-output').textContent='错误: '+e.message}},
async composeAct(p,a){try{const d=await api('/api/docker/compose/'+p+'/'+a,{method:'POST'});$('d-output').textContent=d.output||'完成';toast(a+' 成功','success');setTimeout(()=>D.loadCompose(),800)}catch(e){$('d-output').textContent='错误: '+e.message;toast(a+' 失败','error')}},
async composeLogs(p){try{const d=await api('/api/docker/compose/'+p+'/logs');$('d-output').textContent=d.logs||'(空)'}catch(e){$('d-output').textContent='错误: '+e.message}}};

// Middleware
const MW={types:[{id:'redis',name:'Redis',color:'#fb7185',icon:'R'},{id:'elasticsearch',name:'ES',color:'#fbbf24',icon:'E'},{id:'kafka',name:'Kafka',color:'#34d399',icon:'K'},{id:'nginx',name:'Nginx',color:'#a78bfa',icon:'N'},{id:'tomcat',name:'Tomcat',color:'#6366f1',icon:'T'},{id:'caddy',name:'Caddy',color:'#22d3ee',icon:'C'}],
sel:null,config:null,init(){this.renderTypes();this.loadConfig()},
renderTypes(){$('mw-types').innerHTML=this.types.map(t=>`<div class="mw-t${MW.sel===t.id?' active':''}" onclick="MW.select('${t.id}')"><div class="mw-ic" style="background:${t.color}">${t.icon}</div><span class="mw-nm">${t.name}</span></div>`).join('')},
async loadConfig(){try{this.config=await api('/api/middleware/config');this.renderConns()}catch(e){}},
select(id){this.sel=id;this.renderTypes();$('mw-title').textContent=this.types.find(t=>t.id===id)?.name||id;$('mw-add-btn').style.display='';this.renderConns()},
renderConns(){if(!this.config||!this.sel){$('mw-conn').innerHTML='<div class="mc-empty">暂无</div>';return}
const c=this.config[this.sel]||[];if(!c.length){$('mw-conn').innerHTML='<div class="mc-empty">暂无连接</div>';return}
let h='<table><tr><th>名称</th><th>地址</th><th>操作</th></tr>';c.forEach(x=>{const a=x.host?x.host+':'+(x.port||''):x.brokers||x.config_path||'-';h+=`<tr><td style="font-weight:500">${esc(x.name)}</td><td style="font-family:monospace;font-size:9px">${esc(a)}</td><td><button class="btn btn-sm btn-danger" onclick="MW.remove('${esc(x.name)}')">删除</button></td></tr>`});h+='</table>';$('mw-conn').innerHTML=h},
showAdd(){if(!this.sel)return;const t=this.sel;let f='';
if(t==='redis')f='<div class="form-group"><label>名称</label><input name="name"></div><div class="form-row"><div class="form-group"><label>主机</label><input name="host" value="127.0.0.1"></div><div class="form-group"><label>端口</label><input name="port" value="6379" type="number"></div></div><div class="form-group"><label>密码</label><input name="password" type="password"></div>';
else if(t==='elasticsearch')f='<div class="form-group"><label>名称</label><input name="name"></div><div class="form-row"><div class="form-group"><label>主机</label><input name="host" value="127.0.0.1"></div><div class="form-group"><label>端口</label><input name="port" value="9200" type="number"></div></div>';
else if(t==='kafka')f='<div class="form-group"><label>名称</label><input name="name"></div><div class="form-group"><label>Brokers</label><input name="brokers" value="localhost:9092"></div>';
else if(t==='nginx')f='<div class="form-group"><label>名称</label><input name="name"></div><div class="form-group"><label>配置路径</label><input name="config_path" value="/etc/nginx/nginx.conf"></div>';
else if(t==='tomcat')f='<div class="form-group"><label>名称</label><input name="name"></div><div class="form-group"><label>CATALINA_HOME</label><input name="catalina_home" value="/opt/tomcat"></div>';
else if(t==='caddy')f='<div class="form-group"><label>名称</label><input name="name"></div><div class="form-group"><label>配置路径</label><input name="config_path" value="/etc/caddy/Caddyfile"></div>';
showModal('添加 '+this.types.find(x=>x.id===t)?.name+' 连接',f,()=>MW.add())},
async add(){const f=$('modal-body'),d={};f.querySelectorAll('input').forEach(i=>{if(i.value)d[i.name]=i.name==='port'?parseInt(i.value):i.value});if(!d.name)return toast('输入名称','error');
try{await api('/api/middleware/'+this.sel+'/add',{method:'POST',body:JSON.stringify(d)});toast('已添加','success');closeModal();await this.loadConfig()}catch(e){toast('失败','error')}},
async remove(n){if(!this.sel)return;try{await api('/api/middleware/'+this.sel+'/remove',{method:'POST',body:JSON.stringify({name:n})});toast('已删除','success');await this.loadConfig()}catch(e){toast('失败','error')}},
async discover(){try{const d=await api('/api/middleware/discover');const s=d.services||[];if(!s.length){$('mw-content').innerHTML='<div class="mc-empty">未发现</div>';return}
let h='<table><tr><th>服务</th><th>类型</th><th>端口</th><th>PID</th><th>版本</th></tr>';s.forEach(x=>{h+=`<tr><td style="font-weight:500">${esc(x.name)}</td><td>${esc(x.mw_type)}</td><td style="color:var(--cyan)">${x.port||'-'}</td><td>${x.pid||'-'}</td><td>${esc(x.version||'-')}</td></tr>`});h+='</table>';$('mw-content').innerHTML=h;toast('发现 '+s.length+' 个','success')}catch(e){toast('失败','error')}},
async runCli(){const inp=$('mw-cli'),cmd=inp.value.trim();if(!cmd||!this.sel)return;
if(this.sel==='redis'){const c=this.config?.redis?.[0];if(!c)return toast('先添加 Redis','error');
try{const d=await api('/api/middleware/redis/cli',{method:'POST',body:JSON.stringify({host:c.host,port:c.port,password:c.password,cmd})});$('mw-output').textContent=d.output;inp.value=''}catch(e){$('mw-output').textContent='错误: '+e.message}}else toast('仅支持 Redis CLI','error')}};

// Init
loadPlugins();startDash();
setInterval(async()=>{try{await loadPlugins()}catch(e){}},30000);
