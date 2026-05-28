// DevTool Web UI — Particles, Plugin Management, Upload
const S={plugins:[],sel:null,action:null,loading:false};

// Particle background
(function(){
  const c=document.getElementById('particles'),ctx=c.getContext('2d');
  let w,h,pts=[];
  function rs(){w=c.width=window.innerWidth;h=c.height=window.innerHeight}
  rs();window.addEventListener('resize',rs);
  for(let i=0;i<60;i++)pts.push({x:Math.random()*w,y:Math.random()*h,
    vx:(Math.random()-.5)*.4,vy:(Math.random()-.5)*.4,r:Math.random()*1.5+.4});
  (function dr(){
    ctx.clearRect(0,0,w,h);
    for(let p of pts){
      p.x+=p.vx;p.y+=p.vy;
      if(p.x<0||p.x>w)p.vx*=-1;if(p.y<0||p.y>h)p.vy*=-1;
      ctx.beginPath();ctx.arc(p.x,p.y,p.r,0,Math.PI*2);
      ctx.fillStyle='rgba(96,165,250,0.06)';ctx.fill()}
    for(let i=0;i<pts.length;i++)for(let j=i+1;j<pts.length;j++){
      const d=Math.hypot(pts[i].x-pts[j].x,pts[i].y-pts[j].y);
      if(d<140){ctx.beginPath();ctx.moveTo(pts[i].x,pts[i].y);ctx.lineTo(pts[j].x,pts[j].y);
        ctx.strokeStyle=`rgba(96,165,250,${.025*(1-d/140)})`;ctx.stroke()}}
    requestAnimationFrame(dr)
  })()
})();

async function api(path,opts={}){
  try{const r=await fetch(path,{headers:{'Content-Type':'application/json'},...opts});
    const d=await r.json();if(!r.ok)throw new Error(d.error||'HTTP '+r.status);return d}
  catch(e){toast(e.message,'error');throw e}
}

async function load(){
  try{S.plugins=(await api('/api/plugins')).plugins;
    $('pcount').textContent=S.plugins.length+' plugins';
    renderList();if(S.plugins.length&&!S.sel)select(S.plugins[0].name)}
  catch(e){$('pcount').textContent='disconnected';$('dot').style.background='var(--e)'}
}

function $(id){return document.getElementById(id)}

function renderList(){
  const q=$('search').value.toLowerCase();
  $('plist').innerHTML=S.plugins.filter(p=>!q||p.name.includes(q)||
    p.category.toLowerCase().includes(q)||p.actions.some(a=>a.name.includes(q))
  ).map(p=>`<div class="pi${S.sel===p.name?' active':''}" onclick="select('${p.name}')">
    <span class="ic ${icls(p.category)}">${ich(p.category)}</span>
    <div class="in"><div class="nm">${esc(p.name)}</div><div class="nc">${p.category} · v${p.version}</div></div>
  </div>`).join('')||'<div class="placeholder">No plugins</div>'
}

function select(name){S.sel=name;S.action=null;const p=S.plugins.find(x=>x.name===name);if(!p)return;
  $('dname').textContent=p.name;$('dcat').textContent=p.category;
  $('ddesc').textContent=p.description;
  $('dmeta').innerHTML='<span>v'+p.version+'</span><span>'+p.actions.length+' actions</span>';
  $('agrid').innerHTML=p.actions.length?p.actions.map(a=>
    `<button class="ab${S.action===a.name?' active':''}" onclick="sa('${a.name}')">${esc(a.name)}</button>`
  ).join(''):'<span class="placeholder">No actions</span>';
  if(!S.action&&p.actions.length)sa(p.actions[0].name);renderList()}

function sa(name){S.action=name;document.querySelectorAll('.ab').forEach(b=>b.classList.toggle('active',b.dataset.a===name))}

async function run(){
  if(!S.sel||!S.action)return toast('Select plugin & action','error');
  if(S.loading)return;S.loading=true;
  const btn=$('brun'),ov=$('overlay'),out=$('output');
  btn.disabled=true;btn.textContent='Running...';ov.classList.add('hidden');out.textContent='';
  try{const d=await api('/api/plugins/'+S.sel+'/execute',{method:'POST',
    body:JSON.stringify({action:S.action,input_data:$('input').value||null})});
    out.textContent=d.data||d.error||'(empty)';out.className='co'+(d.success?'':' err');
    $('fmsg').textContent=d.success?'Completed':'Failed';
    toast(d.success?'Done':'Failed: '+(d.error||'unknown'),d.success?'success':'error')}
  catch(e){out.textContent=e.message;out.className='co err';$('fmsg').textContent='Error'}
  finally{S.loading=false;btn.disabled=false;btn.textContent='▶ Run'}}

async function upFile(file){
  if(!file.name.endsWith('.so')&&!file.name.endsWith('.dylib')&&!file.name.endsWith('.dll'))
    {toast('Only .so/.dylib/.dll','error');return}
  toast('Uploading '+file.name+'...');
  try{const buf=await file.arrayBuffer(),
    r=await fetch('/api/plugins/upload?name='+encodeURIComponent(file.name),{method:'POST',body:buf}),
    d=await r.json();
    if(d.status==='uploaded')toast(d.name+' v'+d.version+' loaded','success');
    else toast(d.warning||'saved','error');await load()}
  catch(e){toast(e.message,'error')}}

const uz=$('uzone'),fi=$('fupload');uz.onclick=()=>fi.click();
uz.ondragover=e=>{e.preventDefault();uz.classList.add('drag')};
uz.ondragleave=()=>uz.classList.remove('drag');
uz.ondrop=e=>{e.preventDefault();uz.classList.remove('drag');[...e.dataTransfer.files].forEach(upFile)};
fi.onchange=()=>{[...fi.files].forEach(upFile);fi.value=''};

$('brun').onclick=run;
$('bclear').onclick=()=>{$('input').value='';$('output').textContent='';$('overlay').classList.remove('hidden');$('fmsg').textContent='Cleared'};
$('search').oninput=renderList;
document.onkeydown=e=>{if(e.ctrlKey&&e.key==='Enter'){e.preventDefault();run()};
  if(e.key==='Escape'){$('input').value='';$('output').textContent='';$('overlay').classList.remove('hidden')}};

function toast(msg,type){const c=$('toasts'),el=document.createElement('div');
  el.className='toast '+(type||'');el.textContent=msg;c.appendChild(el);
  setTimeout(()=>{el.style.opacity='0';el.style.transform='translateX(40px)';el.style.transition='all .3s';
    setTimeout(()=>el.remove(),300)},2500)}
function esc(s){return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;')}
function icls(c){const m={'DataTool':'d','SystemTool':'s','Security':'k','Middleware':'m','Script':'r','Network':'n'};return'ic-'+(m[c]||'m')}
function ich(c){const m={'DataTool':'D','SystemTool':'S','Security':'K','Middleware':'M','Script':'R','Network':'N'};return m[c]||'?'}
load();setInterval(async()=>{try{await load()}catch(e){}},30000);
