(function(){
'use strict';
var d=document,root=d.documentElement,reduce=matchMedia('(prefers-reduced-motion: reduce)').matches;

/* theme */
function current(){return root.dataset.theme||(matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light')}
d.querySelectorAll('[data-theme-toggle]').forEach(function(b){
  function label(){var dark=current()==='dark';b.setAttribute('aria-pressed',dark?'true':'false')}
  label();
  b.addEventListener('click',function(){var t=current()==='dark'?'light':'dark';root.dataset.theme=t;try{localStorage.setItem('kt-theme',t)}catch(e){}label()});
});

/* language memory */
d.querySelectorAll('[data-lang]').forEach(function(a){a.addEventListener('click',function(){try{localStorage.setItem('kt-lang',a.dataset.lang)}catch(e){}})});

/* header border */
var hdr=d.querySelector('.site-header');
if(hdr){var onS=function(){hdr.classList.toggle('scrolled',scrollY>8)};addEventListener('scroll',onS,{passive:true});onS()}

/* menu */
var menu=d.querySelector('.menu');
if(menu){d.addEventListener('click',function(e){if(menu.open&&!menu.contains(e.target))menu.open=false});d.addEventListener('keydown',function(e){if(e.key==='Escape'&&menu.open){menu.open=false;menu.querySelector('summary').focus()}})}

/* reveal */
/* visibility: a light scroll check (works everywhere, no observers to miss) */
var watchers=[];
function watch(el,enter,leave,ratio){watchers.push({el:el,enter:enter,leave:leave,r:ratio||.9,v:false})}
var ticking=false;
function check(){ticking=false;var h=innerHeight;for(var i=0;i<watchers.length;i++){var w=watchers[i],b=w.el.getBoundingClientRect(),vis=b.top<h*w.r&&b.bottom>h*(1-w.r);if(vis!==w.v){w.v=vis;if(vis)w.enter();else if(w.leave)w.leave()}}}
function req(){if(!ticking){ticking=true;requestAnimationFrame(check)}}
addEventListener('scroll',req,{passive:true});addEventListener('resize',req);
d.querySelectorAll('[data-reveal]').forEach(function(el){if(reduce){el.classList.add('in');return}watch(el,function(){el.classList.add('in')},null,.94)});

/* the 2B symbol, built so its pixels can move */
var CANVAS='M24.9 128A232.73 232.73 0 0 1 232.73 0L791.27 0A232.73 232.73 0 0 1 1024 232.73L1024 791.27A232.73 232.73 0 0 1 896 999.1L896 896L768 896L768 768L640 768L640 640L512 640L512 512L384 512L384 384L256 384L256 256L128 256L128 128L0 128Z';
var STAR='M745.79 93.13Q756.36 69.82 766.94 93.13L797.77 161.14Q808.35 184.45 833.79 187.3L907.99 195.62Q933.43 198.47 914.53 215.73L859.38 266.07Q840.47 283.33 845.62 308.4L860.65 381.55Q865.8 406.62 843.54 393.98L778.62 357.08Q756.36 344.44 734.11 357.08L669.19 393.98Q646.93 406.62 652.08 381.55L667.1 308.4Q672.26 283.33 653.35 266.07L598.2 215.73Q579.29 198.47 604.74 195.62L678.94 187.3Q704.38 184.45 714.95 161.14Z';
function buildMark(el){
  var px='';
  for(var r=0;r<8;r++)for(var c=0;c<8;c++){var k=r-c;if(k<1||(r===7&&c===0))continue;var z=128*(0.9-0.6*(k-1)/6),x=c*128+(128-z)/2,y=r*128+(128-z)/2;
    px+='<rect x="'+x.toFixed(2)+'" y="'+y.toFixed(2)+'" width="'+z.toFixed(2)+'" height="'+z.toFixed(2)+'" rx="'+(z*.1).toFixed(2)+'" style="--x:'+(k*70)+'px;--y:'+(-k*70)+'px;--sx:'+(-k*16-c*4)+'px;--sy:'+(k*16+r*3)+'px;--d:'+(k*70+c*30)+'ms"></rect>'}
  el.innerHTML='<svg viewBox="0 0 1024 1024" aria-hidden="true" focusable="false"><path fill="currentColor" fill-rule="evenodd" d="'+CANVAS+STAR+'"></path><g fill="currentColor">'+px+'</g></svg>';
  if(reduce)return;
  var rects=el.querySelectorAll('rect');
  function play(){el.classList.remove('go');el.classList.add('pre');el.getBoundingClientRect();el.classList.add('go');el.classList.remove('pre');setTimeout(function(){el.classList.remove('go')},1800)}
  if(el.dataset.mark==='intro'){play()}
  if(el.dataset.mark==='live'){
    var seen=false,timer=null;
    el.classList.add('pre');
    watch(el,function(){if(!seen){seen=true;play()}timer=timer||setInterval(function(){var t=rects[Math.floor(Math.random()*rects.length)];t.classList.add('tw');setTimeout(function(){t.classList.remove('tw')},1300)},1700)},function(){if(timer){clearInterval(timer);timer=null}},.8);
    var on=function(){el.classList.add('scatter')},off=function(){el.classList.remove('scatter')};
    el.addEventListener('pointerenter',on);el.addEventListener('pointerleave',off);
    el.addEventListener('click',play);
  }
}
d.querySelectorAll('[data-mark]').forEach(buildMark);

/* tabs: shelf + gallery */
d.querySelectorAll('[data-tabs]').forEach(function(group){
  var tabs=[].slice.call(group.querySelectorAll('[role=tab]'));
  var stage=group.dataset.tabs?d.getElementById(group.dataset.tabs):null;
  var cap=group.dataset.caption?d.getElementById(group.dataset.caption):null;
  function select(t,focus){
    tabs.forEach(function(x){var on=x===t;x.setAttribute('aria-selected',on);x.tabIndex=on?0:-1;var p=x.getAttribute('aria-controls');p=p&&d.getElementById(p);if(p){p.hidden=!on;if(on&&!reduce){p.classList.remove('anim-in');p.getBoundingClientRect();p.classList.add('anim-in')}}});
    if(stage){stage.querySelectorAll('img[data-shot]').forEach(function(i){i.classList.toggle('on',i.dataset.shot===t.dataset.shot)})}
    if(cap){var s=t.querySelector('span');cap.textContent=s?s.textContent:''}
    if(focus){t.focus();var sc=t.closest('.shelf-scroll,.gal-tabs');if(sc){var r=t.getBoundingClientRect(),sr=sc.getBoundingClientRect();if(r.left<sr.left||r.right>sr.right)sc.scrollTo({left:sc.scrollLeft+r.left-sr.left-sr.width/2+r.width/2,behavior:reduce?'auto':'smooth'})}}
  }
  tabs.forEach(function(t,i){
    t.addEventListener('click',function(){select(t)});
    t.addEventListener('keydown',function(e){var n=null;if(e.key==='ArrowRight'||e.key==='ArrowDown')n=tabs[(i+1)%tabs.length];if(e.key==='ArrowLeft'||e.key==='ArrowUp')n=tabs[(i-1+tabs.length)%tabs.length];if(e.key==='Home')n=tabs[0];if(e.key==='End')n=tabs[tabs.length-1];if(n){e.preventDefault();select(n,true)}});
  });
  select(group.querySelector('[aria-selected=true]')||tabs[0]);
});

/* sharing at home */
var house=d.querySelector('.house');
if(house){
  var qr=house.querySelector('.qr'),cells='';
  var fp=function(r,c){var a=r<3&&c<3,b=r<3&&c>5,e=r>5&&c<3;if(!(a||b||e))return null;var rr=r>5?r-6:r,cc=c>5?c-6:c;return !(rr===1&&cc===1)||true}
  for(var r=0;r<9;r++)for(var c=0;c<9;c++){var f=fp(r,c),on=f!==null?true:((r*7+c*13+r*c)%3!==0);if(f!==null){var rr=r>5?r-6:r,cc=c>5?c-6:c;on=!(rr===1&&cc===1&&false)}cells+='<i'+(on?'':' class="o"')+' style="transition-delay:'+((r+c)*28)+'ms"></i>'}
  qr.innerHTML=cells;
  var steps=[].slice.call(d.querySelectorAll('.steps li'));
  var setStep=function(n){house.dataset.step=n;steps.forEach(function(li,i){li.classList.toggle('on',i===n-1)})};
  if(reduce){setStep(3);steps.forEach(function(li){li.classList.remove('on')})}
  else{
    var step=0,t=null,dur=[0,2200,2400,5200];
    var next=function(){step=step%3+1;setStep(step);t=setTimeout(next,dur[step])};
    watch(house,function(){if(!t)next()},function(){clearTimeout(t);t=null},.75);
    steps.forEach(function(li,i){li.addEventListener('click',function(){clearTimeout(t);step=i;next()})});
  }
}
check();setTimeout(check,300);
})();
