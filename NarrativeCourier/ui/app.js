const $ = (s) => document.querySelector(s);
const $$ = (s) => [...document.querySelectorAll(s)];
const tauriInvoke = window.__TAURI__?.core?.invoke;
const previewMode = !tauriInvoke;
const state = { project: null, projects: [], files: [], busy: false, runtime: null, artifacts: [], deliveryContext: null, deliveries: [], deliveryBusy: false, deliverySelection: null, selectedSceneIds: [], deliveryAssets: [], reviewBusy: false, projectFresh: true };

let projectEpoch=0, deliveryLoadEpoch=0, previewEpoch=0, artifactListEpoch=0, artifactReadEpoch=0, runtimeEpoch=0, formEpoch=0, projectsEpoch=0;
const projectScope=()=>({epoch:projectEpoch,name:state.project?.name});
const currentProject=scope=>scope.epoch===projectEpoch&&scope.name===state.project?.name;
const mutationPending=()=>state.busy||state.deliveryBusy||state.reviewBusy;
function invalidateDeliveryView(){
  ++previewEpoch;state.deliverySelection=null;
  $("#delivery-review-actions").classList.add("hidden");
  $("#delivery-detail").classList.add("hidden");
  $("#delivery-preview").replaceChildren();
}
function resetProjectViews(){
  ++deliveryLoadEpoch;
  state.deliveryContext=null;state.deliveries=[];state.deliveryAssets=[];state.selectedSceneIds=[];state.artifacts=[];
  invalidateDeliveryView();closeArtifacts();$("#delivery-list").replaceChildren();
}
function positiveInteger(selector,label){
  const value=Number($(selector).value);
  if(!Number.isSafeInteger(value)||value<1||value>4294967295)throw new Error(`${label} must be an integer from 1 to 4294967295.`);
  return value;
}


const DEMO = {
  name: "the-last-signal", title: "The Last Signal", status: "in_progress", updated_at: "2026-08-18T10:42:00+09:00", next_node: "step_02",
  stages: [
    ["source","Source","completed"],["architecture","Architecture","in_progress"],["execution","Execution System","pending"],
    ["treatment","Treatment","pending"],["scenes","Scene IR","pending"],["draft","Draft","pending"],
    ["revision","Revision","pending"],["verification","Verification","pending"],["final","Release","pending"]
  ].map(([id,label,status])=>({id,label,status,description:`${label} stage`,outputs:[`${label} artifact`]})),
  doctor:{ok:true,errors:[],warnings:[]}, metadata:{source_format:"original",target_duration:"90",revisions:"0"}, review:null, final_view:null
};
const DEMO_REVIEW = {
  ...DEMO, next_node:"checkpoint_b",
  review:{node:"checkpoint_b",label:"Execution Review",gate_type:"checkpoint",source_title:"Agent Result",allowed:["approve","revise","pause"],revision_targets:[3,4],revision_count:0,max_revisions:2,gate_token:"preview-gate-token",
    source_content:"# STEP 3 — Cinematic System\n\nThe apartment corridor restricts sightlines and makes the hidden handoff physically possible.\n\n# STEP 4 — Character State\n\nMAYA knows the signal is compromised; JAX still believes the channel is secure. Their next choice must preserve that knowledge asymmetry.",
    critique_content:"STRENGTHS\n✓ Space now creates a concrete dramatic constraint.\n✓ Character knowledge states are distinguishable.\n\nCONCERNS\n• Jax's access to the service door needs an upstream state change.\n• Preserve REL-006 when staging Maya's next choice."
  }
};
const DEMO_FINAL = {
  ...DEMO, status:"completed", next_node:null, stages:DEMO.stages.map(s=>({...s,status:"completed"})), review:null,
  final_view:{version:1,revisions:2,word_count:12345,revision_summary:"# STS Final Verification\n\nSTS_STATUS: READY\n\n- Protected source assertions: preserved\n- Material source relations: preserved\n- Knowledge and physical state transitions: consistent\n- Undeclared structural invention: none detected",content:"SCN-001. INT. COFFEE SHOP - MORNING\n\nLENA sits alone at a small table, notebook open. She reads the final line, closes it, and pockets the train ticket beneath the cover.\n\n    LENA   I'm going.\n\nShe leaves before her coffee arrives.\n\nSCN-002. EXT. CITY STREET - MORNING\n\nLena joins the commuter flow toward the station, ticket in hand."
  }, metadata:{source_format:"original",target_duration:"90",revisions:"2"}
};
const DEMO_DELIVERY_CONTEXT = {
  narrative_package_sha256:"0".repeat(64),
  scenes:[
    {id:"SCN-001",content:"SCN-001. INT. COFFEE SHOP - MORNING\n\nLENA sits alone at a small table, notebook open. She reads the final line, closes it, and pockets the train ticket beneath the cover.\n\n    LENA   I'm going."},
    {id:"SCN-002",content:"SCN-002. EXT. CITY STREET - MORNING\n\nLena joins the commuter flow toward the station, ticket in hand."}
  ],
  backends:[{backend_id:"internal-html",version:"sample",operations:["RENDER"],formats:["HTML"]}]
};

async function invoke(command, args={}) {
  if (!tauriInvoke) throw new Error("Tauri runtime is not available in browser preview.");
  return tauriInvoke(command, args);
}

async function checkRuntime(force=false){
  if(previewMode)return {ready:false,codex_found:null,authenticated:null,version:null,message:"Runtime unavailable in browser preview"};
  if(state.runtime&&!force)return state.runtime;
  const ticket=++runtimeEpoch;let runtime;
  try{runtime=await invoke("runtime_status")}catch(error){runtime={ready:false,codex_found:null,authenticated:null,version:null,error:true,message:String(error)}}
  if(ticket===runtimeEpoch){state.runtime=runtime;renderRuntime()}
  return runtime;
}

function renderRuntime(){
  const card=$("#runtime-readiness"), runtime=state.runtime; if(!card||!runtime)return;
  if(runtime.ready){card.classList.add("hidden");card.classList.remove("blocked");return}
  card.classList.remove("hidden");card.classList.add("blocked");card.classList.remove("ready");
  $("#runtime-title").textContent=runtime.error?"Runtime status unavailable":runtime.codex_found?"Codex sign-in required":"Codex CLI required";
  $("#runtime-message").textContent=runtime.message; $("#runtime-badge").textContent=runtime.error?"Retry status check":"Setup required";
}

function setBusy(value, label="Working…", cancellable=false) {
  state.busy=value; document.body.classList.toggle("busy",value);
  const b=$("#run-button"); if(b){b.textContent=value?label:runLabel();b.disabled=value||previewMode||!state.projectFresh;}
  const cancel=$("#cancel-run"); if(cancel) cancel.classList.toggle("hidden",!(value&&cancellable));
}
function showBanner(message, tone="warn") { const b=$("#banner"); b.textContent=message; b.style.borderColor=tone==="error"?"#fda29b":""; b.classList.remove("hidden"); }
function clearBanner(){ $("#banner").classList.add("hidden"); }
function route(name){
  if (!previewMode && name !== "project" && !state.project) { name = "project"; showBanner("Create or open a project first."); }
  if (!previewMode && name === "review" && state.project && !state.project.review) { name = "pipeline"; showBanner("No human review is pending."); }
  if (!previewMode && name === "final" && state.project && !state.project.final_view) { name = "pipeline"; showBanner("The final screenplay is not available yet."); }
  if (!previewMode && name === "deliveries" && state.project && state.project.status !== "completed") { name = "pipeline"; showBanner("Deliveries require a completed STS release."); }
  $$(".screen").forEach(el=>el.classList.toggle("active",el.id===`screen-${name}`));
  $$(".nav-item[data-route]").forEach(el=>el.classList.toggle("active",el.dataset.route===name));
  if(name==="pipeline") renderPipeline(); if(name==="review") renderReview(); if(name==="final") renderFinal(); if(name==="deliveries") renderDeliveries();
}

function fmtDate(s){try{return new Intl.DateTimeFormat(undefined,{dateStyle:"medium",timeStyle:"short"}).format(new Date(s))}catch{return s||"—"}}
function statusText(s){return String(s||"").replaceAll("_"," ")}
function currentStage(project){return project?.stages?.find(s=>s.status==="in_progress")||project?.stages?.find(s=>s.status==="pending")||project?.stages?.at(-1)}
function humanNodeName(id){return String(id||"").replace(/^step_/,"STEP ").replaceAll("_"," ")}
function runLabel(){if(!state.project)return"Run to review"; if(state.project.review)return"Open review"; if(state.project.status==="completed")return"Open final"; if(state.project.status==="paused")return"Resume"; return"Run to review"}

async function loadProjects(){
  if(!tauriInvoke)return;const ticket=++projectsEpoch;
  try{const projects=await invoke("list_projects");if(ticket===projectsEpoch){state.projects=projects;renderRecent()}}
  catch(error){if(ticket===projectsEpoch)showBanner(`Project list refresh failed: ${error}`,"error")}
}

function renderRecent(){
  const box=$("#recent-projects"), list=$("#recent-list"); list.replaceChildren();
  if(!state.projects.length){box.classList.add("hidden");return} box.classList.remove("hidden");
  state.projects.slice(0,5).forEach(p=>{const row=document.createElement("div");row.className="project-row";const btn=document.createElement("button");btn.textContent=p.title;btn.onclick=()=>openProject(p.name);const small=document.createElement("small");small.textContent=`${statusText(p.status)} · ${fmtDate(p.updated_at)}`;row.append(btn,small);list.append(row)});
}
async function openProject(name){
  if(mutationPending()){showBanner("Wait for the current operation or cancel it before changing projects.");return}
  const ticket=++projectEpoch;state.project=null;resetProjectViews();route("project");clearBanner();
  try{
    const project=await invoke("get_project",{name});if(ticket!==projectEpoch)return;
    state.project=project;state.projectFresh=true;route(project.status==="completed"?"final":project.review?"review":"pipeline");
  }catch(error){if(ticket===projectEpoch)showBanner(String(error),"error")}
}

async function importSynopsis(){
  if(previewMode){showBanner("Native document import is available in the desktop app.");return}
  if(mutationPending())return;clearBanner();const form=formEpoch,previous=$("#project-synopsis").value;
  try{
    const imported=await invoke("pick_synopsis_file"); if(!imported||form!==formEpoch||$("#project-synopsis").value!==previous)return;
    const area=$("#project-synopsis"); area.value=imported.content; area.dispatchEvent(new Event("input"));
    showBanner(`Imported synopsis: ${imported.name}`);
  }catch(e){if(form===formEpoch)showBanner(String(e),"error")}
}
async function pickReferences(){
  if(previewMode){showBanner("Native file selection is available in the desktop app.");return}
  if(mutationPending())return;const form=formEpoch;
  try{const files=await invoke("pick_reference_files");if(files!==null&&form===formEpoch)setReferenceFiles(files)}catch(e){if(form===formEpoch)showBanner(String(e),"error")}
}
function setReferenceFiles(files){
  if(!Array.isArray(files)||files.some(file=>typeof file.source_path!=="string"||!file.source_path||typeof file.name!=="string"||!file.name))throw new Error("Reference selection does not match the native file contract.");
  const bad=files.find(file=>!/\.(pdf|docx|txt|md|png|jpe?g|webp|gif)$/i.test(file.name));
  if(bad){showBanner(`Unsupported reference type: ${bad.name}`,"error");return}
  state.files=files.map(({source_path,name,size})=>({source_path,name,size}));clearBanner();
  $("#file-label").textContent=state.files.length?state.files.map(file=>file.name).join(", "):"PDF, DOCX, TXT, MD, or images (max 25 MB each)";
}

async function createProject(ev){
  ev.preventDefault();if(previewMode||mutationPending())return;clearBanner();
  const title=$("#project-title").value,synopsis=$("#project-synopsis").value;
  if(!title.trim()||!synopsis.trim()){showBanner("A title and synopsis are required.","error");return}
  const ticket=++projectEpoch;++formEpoch;setBusy(true,"Creating…");
  try{
    const request={project_name:"",title,synopsis,source_format:$("#source-format").value,target_duration:positiveInteger("#target-duration","Target duration"),genre:$("#genre").value.split(",").map(v=>v.trim()).filter(Boolean),tone:$("#tone").value.trim(),theme_direction:$("#theme-direction").value.trim(),references:state.files.map(file=>({source_path:file.source_path}))};
    const project=await invoke("create_project",{request});if(ticket!==projectEpoch)return;
    resetProjectViews();state.project=project;state.projectFresh=true;route("pipeline");await loadProjects();
  }catch(error){if(ticket===projectEpoch)showBanner(String(error),"error")}finally{setBusy(false)}
}

function renderPipeline(){
  const p=state.project||(previewMode?DEMO:null); if(!p)return; const stage=currentStage(p);
  $("#project-meta").innerHTML=`<span>Project</span><strong>${escapeHtml(p.title)}</strong><span>Current step</span><strong>${escapeHtml(stage?.label||"Complete")}</strong><span>Status</span><strong>${escapeHtml(state.projectFresh?statusText(p.status):`Last loaded: ${statusText(p.status)}`)}</strong><span>Last updated</span><strong>${escapeHtml(fmtDate(p.updated_at))}</strong>`;
  const list=$("#stage-list");list.replaceChildren();p.stages.forEach((s,i)=>{const row=document.createElement("div");row.className=`stage-row ${s.status} ${s===stage?"current":""}`;row.innerHTML=`<span class="stage-dot">${s.status==="completed"?"✓":""}</span><div><div class="stage-name">${i+1}. ${escapeHtml(s.label)}</div><div class="stage-status">${escapeHtml(statusText(s.status))}</div></div>`;list.append(row)});
  $("#stage-badge").textContent=p.status==="completed"?"Complete":`${Math.max(1,p.stages.findIndex(s=>s===stage)+1)} of ${p.stages.length}`;
  $("#stage-title").textContent=stage?.label||"Complete";
  const info=stage||{description:"The screenplay workflow is complete.",outputs:["Final screenplay"]};
  $("#stage-description").textContent=p.review?"A human decision is required before the workflow can continue.":info.description;
  $("#stage-state").textContent=previewMode?"Sample workflow snapshot; integrity has not been executed.":!state.projectFresh?"Current project state is unavailable; reopen this project to refresh.":!p.doctor?"Integrity status unavailable":p.doctor.ok?`Project integrity verified.${p.next_node?` Next: ${humanNodeName(p.next_node)}.`:""}`:`Integrity check failed: ${(p.doctor?.errors||[]).join("; ")}`;
  const outputs=$("#stage-outputs");outputs.replaceChildren();info.outputs.forEach(v=>{const li=document.createElement("li");li.textContent=v;outputs.append(li)});
  $("#next-action-text").textContent=p.review?"Review the current result and approve or request a revision.":p.status==="completed"?"Open the final screenplay.":p.status==="paused"?"Resume the project before running agents.":"Run Agent obligations until the next Human decision is required.";
  $("#review-artifacts").disabled=false; $("#review-artifacts").onclick=openArtifacts; $("#run-button").textContent=runLabel();$("#run-button").disabled=previewMode||state.busy||!state.projectFresh;
}
async function runProject(){
  if(previewMode){showBanner("Browser preview cannot run workflow effects.");return}
  if(!state.project||!state.projectFresh||mutationPending())return;
  if(state.project.review)return route("review");if(state.project.status==="completed")return route("final");
  const scope=projectScope();clearBanner();setBusy(true,"Checking runtime…");
  try{
    const runtime=await checkRuntime(true);if(!currentProject(scope))return;
    if(!runtime.ready){showBanner(runtime.message,"error");return}
    setBusy(true,state.project.status==="paused"?"Resuming…":"Running…",true);
    if(state.project.status==="paused"){const resumed=await invoke("resume_project",{name:scope.name});if(!currentProject(scope))return;state.project=resumed;state.projectFresh=true;}
    for(let index=0;index<32;index++){
      if(!currentProject(scope))return;renderPipeline();
      $("#stage-state").classList.add("run-progress");$("#stage-state").textContent=`Running ${humanNodeName(state.project.next_node)}…`;
      const result=await invoke("run_next_agent",{name:scope.name});if(!currentProject(scope))return;
      state.project=result.project;state.projectFresh=true;renderPipeline();await new Promise(resolve=>requestAnimationFrame(resolve));
      if(state.project.review||state.project.status==="completed"||state.project.status==="paused")break;
    }
    if(currentProject(scope))route(state.project.status==="completed"?"final":state.project.review?"review":"pipeline");
  }catch(error){
    if(currentProject(scope)){
      try{const project=await invoke("get_project",{name:scope.name});if(currentProject(scope)){state.project=project;state.projectFresh=true;renderPipeline()}}catch(reloadError){state.projectFresh=false;renderPipeline();showBanner(`${error}; current project reload failed: ${reloadError}`,"error");return}
      showBanner(String(error),"error");
    }
  }finally{$("#stage-state")?.classList.remove("run-progress");setBusy(false)}
}

async function cancelRun(){
  if(previewMode||!state.project||!state.busy)return;
  const scope=projectScope();
  try{await invoke("cancel_agent_run",{name:scope.name});if(currentProject(scope))$("#stage-state").textContent="Cancellation requested; waiting for the Agent outcome…"}
  catch(error){if(currentProject(scope))showBanner(String(error),"error")}
}

function renderReview(){
  const p=state.project?.review?state.project:(previewMode?DEMO_REVIEW:null), r=p?.review; if(!r){showBanner("No human review is pending.");return}
  $("#review-title").textContent=r.label||"Review"; $("#review-chip").textContent=r.gate_type==="checkpoint"?r.label:"Human review"; $("#review-source-title").textContent=r.source_title||"Agent Result"; $("#review-source").textContent=r.source_content||"No artifact preview.";
  const critCard=$("#critic-review-card"), crit=$("#review-critique");
  critCard.classList.toggle("hidden",!r.critique_content); if(r.critique_content)crit.textContent=r.critique_content;
  $(".review-layout").classList.toggle("single-review",!r.critique_content);
  $("#feedback-label").textContent="Revision request"; $("#feedback-hint").textContent="(when revising)";
  const targetWrap=$("#revision-target-wrap"), target=$("#revision-target"); target.replaceChildren(); (r.revision_targets||[]).forEach(s=>{const o=document.createElement("option");o.value=s;o.textContent=`STEP ${s}`;target.append(o)}); targetWrap.classList.toggle("hidden",!(r.revision_targets||[]).length);
  const buttons=$("#decision-buttons");buttons.replaceChildren(); const allowed=r.allowed||[];
  if(allowed.includes("approve")) buttons.append(decisionButton("✓  Approve","approve","approve"));
  if(allowed.includes("revise")) buttons.append(decisionButton("↻  Request Revision","revise","revise"));
  if(allowed.includes("pause")){const b=decisionButton("Pause","pause","secondary");b.classList.add("secondary");buttons.append(b)}
}
function decisionButton(label,decision,cls){const b=document.createElement("button");b.className=`btn ${cls}`;b.textContent=label;b.disabled=previewMode||!state.projectFresh||mutationPending();b.onclick=()=>decide(decision);return b}
async function decide(decision){
  const project=state.project,review=project?.review;if(previewMode||!project||!review||!state.projectFresh||mutationPending())return;
  const scope=projectScope(),feedback=$("#revision-feedback").value.trim();let step=null;
  if(decision==="revise"&&review.revision_targets?.length)step=Number($("#revision-target").value);
  if(decision==="revise"&&!feedback){showBanner("A revision request is required when revising.");return}
  let applied=false;setBusy(true,"Saving decision…");clearBanner();
  try{
    await invoke("decide_review",{name:scope.name,decision,step,feedback:feedback||null,gateToken:review.gate_token});applied=true;
    const updated=await invoke("get_project",{name:scope.name});if(!currentProject(scope))return;
    state.project=updated;state.projectFresh=true;$("#revision-feedback").value="";route("pipeline");
  }catch(error){
    if(currentProject(scope)){
      if(applied){state.projectFresh=false;$("#decision-buttons").replaceChildren();route("pipeline")}
      showBanner(applied?`Decision saved; reload failed: ${error}`:String(error),"error");
    }
  }finally{setBusy(false)}
}

function renderFinal(){
  const p=state.project?.final_view?state.project:(previewMode?DEMO_FINAL:null), f=p?.final_view; if(!f){showBanner("Final screenplay is not available yet.");return}
  $("#screenplay-viewer").textContent=f.content; $("#final-words").textContent=`${Number(f.word_count||0).toLocaleString()} words`; $("#final-pages").textContent=`Version ${f.version}`;
  $("#final-summary").innerHTML=`<dt>Project title</dt><dd>${escapeHtml(p.title)}</dd><dt>Version</dt><dd>${f.version}</dd><dt>Last updated</dt><dd>${escapeHtml(fmtDate(p.updated_at))}</dd><dt>Target runtime</dt><dd>${escapeHtml(p.metadata?.target_duration||"—")} min</dd>`;
  const completion=$("#completion-status");completion.replaceChildren();p.stages.filter(s=>s.status==="completed").slice(-4).forEach(s=>{const d=document.createElement("div");d.className="completion-row";d.innerHTML=`<span class="completion-check">✓</span><span>${escapeHtml(s.label)} completed</span>`;completion.append(d)});
  $("#revision-count").textContent=f.revisions?`${f.revisions} Human-directed revision${f.revisions===1?"":"s"}`:"No Human-requested revisions";
  $("#revision-summary").textContent=f.revision_summary||"STEP 8 revision evidence is unavailable.";
}

function canonicalScenes(){return state.deliveryContext?.scenes||[]}
function ensureSceneSelection(){
  const valid=new Set(canonicalScenes().map(scene=>scene.id));
  state.selectedSceneIds=state.selectedSceneIds.filter(id=>valid.has(id));
  if(!state.selectedSceneIds.length&&valid.size)state.selectedSceneIds=[...valid];
}
function selectedScenes(){
  const selected=new Set(state.selectedSceneIds);
  return canonicalScenes().filter(scene=>selected.has(scene.id));
}
function sceneAssetCount(sceneId){return state.deliveryAssets.filter(asset=>asset.scene_refs?.includes(sceneId)).length}
function renderStoryboard(){
  const rail=$("#storyboard-scenes"), scenes=canonicalScenes(); if(!rail)return;
  ensureSceneSelection(); rail.replaceChildren();
  if(!scenes.length){
    const empty=document.createElement("p");empty.className="empty-state";empty.textContent="No canonical scenes are available. A verified NarrativePackage is required.";rail.append(empty);
  }else{
    const selected=new Set(state.selectedSceneIds);
    scenes.forEach((scene,index)=>{
      const card=document.createElement("button");card.type="button";card.className="scene-card";card.dataset.sceneId=scene.id;card.setAttribute("role","option");card.setAttribute("aria-selected",selected.has(scene.id));card.tabIndex=selected.has(scene.id)?0:-1;
      const imageAsset=state.deliveryAssets.find(asset=>asset.scene_refs?.includes(scene.id));
      const media=document.createElement("span");media.className="scene-thumb";
      if(imageAsset?.preview_data_url){const img=document.createElement("img");img.src=imageAsset.preview_data_url;img.alt=`Approved external visual for ${scene.id}`;media.append(img)}else{media.textContent=`${String(index+1).padStart(2,"0")}`}
      const copy=document.createElement("span");copy.className="scene-card-copy";
      const title=document.createElement("strong");title.textContent=scene.id;const content=document.createElement("span");content.textContent=scene.content;copy.append(title,content);
      const meta=document.createElement("small");meta.textContent=sceneAssetCount(scene.id)?"Image attached · fit unverified":"No image attached";copy.append(meta);card.append(media,copy);
      card.addEventListener("click",()=>toggleScene(scene.id));
      card.addEventListener("keydown",event=>{if(event.key==="ArrowRight"||event.key==="ArrowDown"){event.preventDefault();rail.querySelectorAll(".scene-card")[(index+1)%scenes.length].focus()}if(event.key==="ArrowLeft"||event.key==="ArrowUp"){event.preventDefault();rail.querySelectorAll(".scene-card")[(index-1+scenes.length)%scenes.length].focus()}if(event.key===" "||event.key==="Enter"){event.preventDefault();toggleScene(scene.id)}});
      rail.append(card);
    });
  }
  const selected=selectedScenes();
  $("#storyboard-selection-label").textContent=selected.length?`${selected.length} scene${selected.length===1?"":"s"} selected`:(scenes.length?"No scenes selected":"No scenes available");
  $("#storyboard-selection-help").textContent=previewMode?"Preview-only sample. Import controls are disabled in the browser.":"Scene text is read from the verified NarrativePackage; no summary or prompt is invented.";
  const status=$("#storyboard-status");status.classList.toggle("is-preview",previewMode);status.textContent=previewMode?"Preview only · import disabled":scenes.length?"Verified release · ready for visual review":"Waiting for a verified release";
  $("#copy-scene-brief").disabled=!selected.length;
  $("#pick-delivery-asset").disabled=previewMode||!selected.length||state.deliveryBusy||state.reviewBusy||!state.projectFresh||!state.project;
  renderStoryboardAssets();
  const coverage=$("#delivery-coverage");
  if(coverage&&selected.length){coverage.value=selected.length===scenes.length?"FULL":(coverage.value==="EXCERPT"?"EXCERPT":"SELECTION")}
  deliveryCoverageConfig();
  deliveryFormatConfig();
}
function toggleScene(sceneId){
  const selected=new Set(state.selectedSceneIds); if(selected.has(sceneId))selected.delete(sceneId);else selected.add(sceneId);
  state.selectedSceneIds=[...selected]; renderStoryboard();
}
function renderStoryboardAssets(){
  const box=$("#storyboard-assets"); if(!box)return; box.replaceChildren();
  if(!state.deliveryAssets.length){const empty=document.createElement("p");empty.className="muted-copy";empty.textContent=previewMode?"No imported file in preview. The sample visuals above are placeholders.":"No external image reference attached. Select scenes, then attach a file to stage it.";box.append(empty);$("#delivery-image-policy").textContent="No image attached";return}
  $("#delivery-image-policy").textContent=`HTML only · ${state.deliveryAssets.length} image${state.deliveryAssets.length===1?"":"s"}`;
  state.deliveryAssets.forEach(asset=>{
    const card=document.createElement("article");card.className="asset-chip";
    const img=document.createElement("img");img.src=asset.preview_data_url;img.alt=`Preview of ${asset.name}`;
    const copy=document.createElement("div");const name=document.createElement("strong");name.textContent=asset.name;const meta=document.createElement("span");meta.textContent=`${asset.source?.sha256?.slice(0,12)||"—"}… · ${asset.width}×${asset.height} · ${formatBytes(asset.size_bytes)}`;const note=document.createElement("small");note.textContent=`External image · narrative fit unverified · ${asset.scene_refs.join(", ")}`;copy.append(name,meta,note);
    const remove=document.createElement("button");remove.type="button";remove.className="btn text asset-remove";remove.textContent="Remove";remove.addEventListener("click",()=>{state.deliveryAssets=state.deliveryAssets.filter(item=>item.asset_id!==asset.asset_id);renderStoryboard()});card.append(img,copy,remove);box.append(card);
  });
}
function formatBytes(value){const bytes=Number(value||0);if(bytes<1024)return`${bytes} B`;if(bytes<1024*1024)return`${(bytes/1024).toFixed(1)} KiB`;return`${(bytes/(1024*1024)).toFixed(1)} MiB`}
function deliveryFormatConfig(){
  const select=$("#delivery-format"), hasImages=state.deliveryAssets.length>0;
  if(hasImages&&select.value!=="HTML")select.value="HTML";
  [...select.options].forEach(option=>{option.disabled=hasImages&&option.value!=="HTML"});
  const format=select.value, image=["PNG","WEBP"].includes(format), video=format==="MP4";
  $$(".delivery-dim").forEach(el=>el.classList.toggle("hidden",!(image||video)));
  $$(".delivery-video").forEach(el=>el.classList.toggle("hidden",!video));
  const compatible=(state.deliveryContext?.backends||[]).filter(b=>(b.operations||[]).includes("RENDER")&&(b.formats||[]).includes(format));
  const backend=$("#delivery-backend"), current=backend.value;backend.replaceChildren(new Option("Auto",""));
  compatible.forEach(b=>backend.append(new Option(`${b.backend_id} · ${b.version}`,b.backend_id)));
  if([...backend.options].some(o=>o.value===current))backend.value=current;
  const ambiguous=compatible.length>1&&!backend.value;
  const policy=ambiguous?"Select a backend for this output":hasImages?"HTML only · attached images require Human preview":(compatible.length?`${compatible.length} compatible backend${compatible.length===1?"":"s"}`:`No available backend for ${format}`);
  $("#delivery-capability").textContent=previewMode?`${policy} · Generate disabled in preview`:policy;
  $("#run-delivery").disabled=previewMode||ambiguous||!compatible.length||state.deliveryBusy||state.reviewBusy||!state.projectFresh||!state.deliveryContext||!selectedScenes().length;
}
function deliveryCoverageConfig(){
  const coverage=$("#delivery-coverage"), scenes=canonicalScenes();
  if(coverage.value==="FULL")state.selectedSceneIds=scenes.map(scene=>scene.id);
  else state.selectedSceneIds=state.selectedSceneIds.filter(id=>scenes.some(scene=>scene.id===id));
  const selected=selectedScenes();
  $("#delivery-scenes-wrap").classList.toggle("hidden",coverage.value==="FULL");
  $("#delivery-scenes").value=coverage.value==="FULL"?"":selected.map(scene=>scene.id).join(", ");
}
function makeDeliveryRequest(){
  deliveryCoverageConfig();
  const format=$("#delivery-format").value, coverage=$("#delivery-coverage").value, scenes=selectedScenes();
  if(!scenes.length)throw new Error("Select at least one canonical SCN-* scene.");
  const refs=coverage==="FULL"?[]:scenes.map(scene=>scene.id);
  const selectedIds=new Set(scenes.map(scene=>scene.id));
  const asset_inputs=state.deliveryAssets.map(asset=>{
    const narrative_refs=asset.scene_refs.filter(id=>selectedIds.has(id));
    if(!narrative_refs.length)throw new Error(`${asset.name} is not scoped to a selected SCN-* scene.`);
    return {asset_id:asset.asset_id,kind:"IMAGE",role:"external visual reference",narrative_refs,generation_mode:"USER",source:asset.source};
  });
  if(asset_inputs.length&&format!=="HTML")throw new Error("Image-backed deliveries are HTML-only.");
  const output={output_id:`OUT-${Date.now()}`,format,constraints:[]};
  const backendId=$("#delivery-backend").value;
  if(backendId)output.backend_id=backendId;
  if(["PNG","WEBP","MP4"].includes(format)){
    output.width=positiveInteger("#delivery-width","Width");
    output.height=positiveInteger("#delivery-height","Height");
  }
  if(format==="MP4"){
    output.fps={num:positiveInteger("#delivery-fps","FPS"),den:1};
    output.duration_frames=positiveInteger("#delivery-frames","Frame count");
  }
  return {schema_version:1,request_id:`REQ-${Date.now()}`,narrative_package_sha256:state.deliveryContext.narrative_package_sha256,intent:"PRESENTATION",coverage,must_include_refs:refs,constraints:[],asset_inputs,outputs:[output]};
}
async function copySceneBrief(){
  const scene=selectedScenes()[0];if(!scene)return;
  const brief=`Source scene below is canonical. Create a visual draft without adding narrative facts.\n\n${scene.content}`;
  try{await navigator.clipboard.writeText(brief);showBanner("Canonical scene brief copied. The instruction adds no narrative meaning.")}catch{showBanner(brief)}
}
async function pickDeliveryAsset(){
  const scenes=selectedScenes();if(previewMode||!scenes.length||!state.projectFresh||mutationPending())return;
  const scope=projectScope(),packageHash=state.deliveryContext?.narrative_package_sha256;
  const refs=scenes.map(scene=>scene.id);
  try{
    const imported=await invoke("pick_delivery_asset",{name:scope.name});
    if(imported===null||!currentProject(scope)||packageHash!==state.deliveryContext?.narrative_package_sha256)return;
    state.deliveryAssets=state.deliveryAssets.filter(asset=>asset.asset_id!==imported.asset_id);
    state.deliveryAssets.push({...imported,scene_refs:refs});renderStoryboard();
    showBanner(`Staged ${imported.name}. Narrative fit remains unverified until Human review.`);
  }catch(error){if(currentProject(scope))showBanner(String(error),"error")}
}

async function renderDeliveries(){
  const project=state.project||(previewMode?DEMO_FINAL:null);if(!project||mutationPending())return;
  const scope=projectScope(),ticket=++deliveryLoadEpoch;
  try{
    let context,deliveries;
    if(previewMode){context=DEMO_DELIVERY_CONTEXT;deliveries=[]}
    else{
      context=await invoke("delivery_context",{name:scope.name});
      deliveries=await invoke("list_deliveries",{name:scope.name});
    }
    if(!currentProject(scope)||ticket!==deliveryLoadEpoch)return;
    if(state.deliveryContext&&state.deliveryContext.narrative_package_sha256!==context.narrative_package_sha256){
      state.deliveryAssets=[];state.selectedSceneIds=[];invalidateDeliveryView();
      showBanner("NarrativePackage changed; staged visuals were cleared until you review the new release.");
    }
    state.deliveryContext=context;state.deliveries=deliveries;
    $("#delivery-package").innerHTML=previewMode?"<span>Mode</span><strong>Browser preview</strong><span>Narrative</span><strong>Sample scenes only</strong>":`<span>Project</span><strong>${escapeHtml(project.title)}</strong><span>Narrative</span><strong>${escapeHtml(context.narrative_package_sha256.slice(0,12))}…</strong><span>Scenes</span><strong>${context.scenes.length}</strong>`;
    ensureSceneSelection();renderStoryboard();deliveryFormatConfig();deliveryCoverageConfig();renderDeliveryList();
  }catch(error){
    if(!currentProject(scope)||ticket!==deliveryLoadEpoch)return;
    state.deliveryContext=null;state.deliveries=[];invalidateDeliveryView();renderDeliveryList();
    $("#delivery-package").textContent="Current delivery context unavailable";
    $("#storyboard-scenes").replaceChildren();$("#storyboard-status").textContent="Unable to load canonical scenes";
    deliveryFormatConfig();showBanner(String(error),"error");
  }
}

function renderDeliveryList(){
  const list=$("#delivery-list"); list.replaceChildren();
  if(!state.deliveries.length){const p=document.createElement("p");p.className="muted-copy";p.textContent=state.deliveryContext?"No deliveries yet.":"Delivery records unavailable; refresh to reload them.";list.append(p);return}
  state.deliveries.slice().reverse().forEach(record=>{
    const set=record.set, b=document.createElement("button");b.className="delivery-item";
    b.innerHTML=`<strong>${escapeHtml(record.request_id)}</strong><span>${escapeHtml(statusText(set.status))} · ${set.deliveries.length} output${set.deliveries.length===1?"":"s"}</span>`;
    b.onclick=()=>{if(!state.reviewBusy)showDeliverySet(record)};list.append(b);
  });
}
async function runDelivery(){
  if(previewMode||!state.project||!state.projectFresh||!state.deliveryContext||mutationPending())return;
  clearBanner();let request;try{request=makeDeliveryRequest()}catch(error){showBanner(String(error),"error");return}
  const scope=projectScope(),selectionTicket=++previewEpoch;state.deliverySelection=null;$("#delivery-review-actions").classList.add("hidden");++deliveryLoadEpoch;state.deliveryBusy=true;
  $("#run-delivery").disabled=true;$("#cancel-delivery").classList.remove("hidden");$("#run-delivery").textContent="Generating…";
  try{
    const set=await invoke("run_delivery",{name:scope.name,request});if(!currentProject(scope))return;
    if(selectionTicket===previewEpoch)showDeliverySet({request_id:request.request_id,set});
    else showBanner(`Delivery committed: ${request.request_id}`);
    try{const records=await invoke("list_deliveries",{name:scope.name});if(currentProject(scope)){state.deliveries=records;renderDeliveryList()}}
    catch(error){if(currentProject(scope))showBanner(`Delivery committed; list refresh failed: ${error}`,"error")}
  }catch(error){if(currentProject(scope))showBanner(String(error),"error")}
  finally{state.deliveryBusy=false;$("#cancel-delivery").classList.add("hidden");$("#run-delivery").textContent="Generate";deliveryFormatConfig()}
}

async function cancelDelivery(){
  if(previewMode||!state.project||!state.deliveryBusy)return;
  const scope=projectScope();
  try{await invoke("cancel_delivery",{name:scope.name});if(currentProject(scope))$("#delivery-capability").textContent="Cancellation requested; waiting for the delivery outcome…"}
  catch(error){if(currentProject(scope))showBanner(String(error),"error")}
}

function showDeliverySet(record){
  const set=record.set; ++previewEpoch;state.deliverySelection=null;$("#delivery-preview").replaceChildren();
  $("#delivery-review-actions").classList.add("hidden");
  $("#delivery-detail").classList.remove("hidden");$("#delivery-detail-title").textContent=record.request_id;$("#delivery-detail-meta").textContent=`${set.set_id} · Plan ${set.director_plan_sha256.slice(0,12)}…`;$("#delivery-detail-status").textContent=statusText(set.status);
  const box=$("#delivery-receipts");box.replaceChildren();
  set.deliveries.forEach(receipt=>{
    const card=document.createElement("button");card.className="delivery-receipt";
    const finding=[...receipt.verification.semantic.findings,...receipt.verification.visual.findings,...receipt.verification.technical.findings].join(" · ");
    card.innerHTML=`<strong>${escapeHtml(receipt.output_request_id)} · ${escapeHtml(receipt.artifact.kind)}</strong><span>${escapeHtml(receipt.backend_id)} · ${escapeHtml(statusText(receipt.status))}</span><small>${escapeHtml(finding||"No findings")}</small>`;
    card.onclick=()=>previewDelivery(record,receipt);box.append(card);
  });
}
async function previewDelivery(record,receipt){
  if(state.reviewBusy)return;
  const scope=projectScope(),ticket=++previewEpoch,box=$("#delivery-preview");
  state.deliverySelection=null;$("#delivery-review-actions").classList.add("hidden");box.textContent="Loading preview…";
  try{
    const preview=await invoke("read_delivery_preview",{name:scope.name,requestId:record.request_id,outputId:receipt.output_request_id});
    if(!currentProject(scope)||ticket!==previewEpoch)return;
    renderPreviewValue(box,preview);state.deliverySelection={record,receipt,scope};
    $("#delivery-review-actions").classList.toggle("hidden",receipt.status!=="NEEDS_REVIEW");
  }catch(error){
    if(!currentProject(scope)||ticket!==previewEpoch)return;
    state.deliverySelection=null;box.textContent=String(error);$("#delivery-review-actions").classList.add("hidden");
  }
}

async function reviewDelivery(approve){
  const selected=state.deliverySelection;if(previewMode||!selected||!state.projectFresh||!currentProject(selected.scope)||mutationPending())return;
  let note=null;if(!approve){note=window.prompt("Why is this artifact rejected?","");if(note===null)return}
  const scope=selected.scope;state.reviewBusy=true;++deliveryLoadEpoch;
  $("#approve-delivery-review").disabled=true;$("#reject-delivery-review").disabled=true;deliveryFormatConfig();
  try{
    const set=await invoke("review_delivery",{name:scope.name,requestId:selected.record.request_id,outputId:selected.receipt.output_request_id,approve,note:note||null});
    if(!currentProject(scope))return;showDeliverySet({request_id:selected.record.request_id,set});
    showBanner(approve?"Human review approved.":"Human review rejected.");
    try{const records=await invoke("list_deliveries",{name:scope.name});if(currentProject(scope)){state.deliveries=records;renderDeliveryList()}}
    catch(error){if(currentProject(scope))showBanner(`Review saved; list refresh failed: ${error}`,"error")}
  }catch(error){if(currentProject(scope))showBanner(String(error),"error")}
  finally{state.reviewBusy=false;$("#approve-delivery-review").disabled=false;$("#reject-delivery-review").disabled=false;deliveryFormatConfig()}
}

function renderPreviewValue(box,preview){
  box.replaceChildren();
  if(preview.text!=null){
    if(preview.mime==="text/html"){const frame=document.createElement("iframe");frame.setAttribute("sandbox","");frame.srcdoc=preview.text;frame.title="Delivery HTML preview";box.append(frame)}
    else if(preview.mime==="application/json"){renderJsonPreview(box,preview)}
    else{const pre=document.createElement("pre");pre.textContent=preview.text;box.append(pre)}
    return;
  }
  if(preview.mime.startsWith("image/")){const img=document.createElement("img");img.src=preview.data_url;img.alt="Delivery preview";box.append(img);return}
  if(preview.mime.startsWith("video/")){const video=document.createElement("video");video.src=preview.data_url;video.controls=true;box.append(video);return}
  if(preview.mime==="application/pdf"){const embed=document.createElement("embed");embed.src=preview.data_url;embed.type=preview.mime;box.append(embed);return}
  box.textContent=`Preview unavailable for ${preview.mime}`;
}

function renderJsonPreview(box,preview){
  const showRawFailure=error=>{
    box.replaceChildren();const message=document.createElement("p");message.className="muted-copy";
    message.textContent=`Graph view unavailable: ${error}. Raw JSON is retained below.`;
    const raw=document.createElement("pre");raw.textContent=preview.text;box.append(message,raw);
  };
  let model;
  try{model=NarrativeCourierGraphViewer.parseGraph(preview.text)}catch(error){showRawFailure(error);return}
  const toggleRow=document.createElement("div");toggleRow.className="graph-toggle-row";
  const graphBtn=document.createElement("button");graphBtn.type="button";graphBtn.className="inline-action";graphBtn.setAttribute("aria-pressed","true");graphBtn.textContent="Scene graph";
  const rawBtn=document.createElement("button");rawBtn.type="button";rawBtn.className="inline-action";rawBtn.textContent="Raw JSON";
  toggleRow.append(graphBtn,rawBtn);
  const holder=document.createElement("div");
  const rawWrap=document.createElement("pre");rawWrap.classList.add("hidden");rawWrap.textContent=preview.text;
  const detail=document.createElement("div");detail.className="graph-detail hidden";
  box.append(toggleRow,holder,rawWrap,detail);
  let rendered=false,selectedNode=null;
  const showGraph=()=>{
    holder.classList.remove("hidden");rawWrap.classList.add("hidden");detail.classList.toggle("hidden",selectedNode===null);
    graphBtn.setAttribute("aria-pressed","true");rawBtn.removeAttribute("aria-pressed");
    if(rendered)return;
    try{NarrativeCourierGraphViewer.render(holder,model,{onSelect:(node)=>{
      selectedNode=node;detail.classList.remove("hidden");detail.replaceChildren();
      const heading=document.createElement("strong");heading.textContent=node.refs[0];
      const refs=document.createElement("small");refs.textContent=`${node.id} · ${node.refs.join(" · ")}`;
      const body=document.createElement("pre");body.textContent=node.content===null?"Content omitted":node.content;
      detail.append(heading,refs,body);
    }});rendered=true}catch(error){showRawFailure(error)}
  };
  const showRaw=()=>{
    holder.classList.add("hidden");detail.classList.add("hidden");rawWrap.classList.remove("hidden");
    rawBtn.setAttribute("aria-pressed","true");graphBtn.removeAttribute("aria-pressed");
  };
  graphBtn.addEventListener("click",showGraph);rawBtn.addEventListener("click",showRaw);
  showGraph();
}

async function openArtifacts(){
  if(previewMode||!state.project){showBanner("Artifact browsing requires an open desktop project.");return}
  const scope=projectScope(),ticket=++artifactListEpoch;
  try{
    const artifacts=await invoke("list_artifacts",{name:scope.name});if(!currentProject(scope)||ticket!==artifactListEpoch)return;
    state.artifacts=artifacts;const list=$("#artifact-list");list.replaceChildren();
    for(const item of artifacts){const button=document.createElement("button");button.className="artifact-item";button.innerHTML=`<strong>${escapeHtml(artifactLabel(item))}</strong><small>v${item.version} · ${escapeHtml(item.role)}</small>`;button.onclick=()=>loadArtifact(item.node,button);list.append(button)}
    $("#artifact-drawer").classList.remove("hidden");$("#drawer-scrim").classList.remove("hidden");
    if(artifacts.length)await loadArtifact(artifacts.at(-1).node,list.lastElementChild);else $("#artifact-content").textContent="No completed artifacts yet.";
  }catch(error){if(currentProject(scope)&&ticket===artifactListEpoch)showBanner(String(error),"error")}
}

async function loadArtifact(node,button){
  const scope=projectScope(),ticket=++artifactReadEpoch;$("#artifact-content").textContent="Loading artifact…";
  try{
    const artifact=await invoke("read_artifact",{name:scope.name,node});if(!currentProject(scope)||ticket!==artifactReadEpoch)return;
    $$(".artifact-item").forEach(item=>item.classList.remove("active"));button?.classList.add("active");$("#artifact-content").textContent=artifact.content;
  }catch(error){if(currentProject(scope)&&ticket===artifactReadEpoch){$("#artifact-content").textContent=String(error);showBanner(String(error),"error")}}
}

function closeArtifacts(){++artifactListEpoch;++artifactReadEpoch;$("#artifact-drawer").classList.add("hidden");$("#drawer-scrim").classList.add("hidden")}
function artifactLabel(item){const step=item.step==null?item.node:`STEP ${item.step}`;return item.phase?`${step} · ${String(item.phase).replaceAll("_"," ")}`:step}
async function exportDocx(){
  if(previewMode||!state.project||!state.projectFresh||mutationPending())return;
  const scope=projectScope();setBusy(true,"Exporting…");
  try{const path=await invoke("export_docx",{name:scope.name});if(currentProject(scope))showBanner(`DOCX saved: ${path}`)}
  catch(error){if(currentProject(scope))showBanner(String(error),"error")}finally{setBusy(false)}
}

function escapeHtml(s){return String(s??"").replace(/[&<>'"]/g,c=>({"&":"&amp;","<":"&lt;",">":"&gt;","'":"&#39;",'"':"&quot;"}[c]))}

$$('[data-route]').forEach(b=>b.addEventListener('click',()=>route(b.dataset.route)));
$("#project-form").addEventListener("submit",createProject); $("#run-button").addEventListener("click",runProject); $("#cancel-run").addEventListener("click",cancelRun); $("#export-docx").addEventListener("click",exportDocx);
$("#import-synopsis").addEventListener("click",importSynopsis); $("#dropzone").addEventListener("click",pickReferences);
$("#close-artifacts").addEventListener("click",closeArtifacts); $("#drawer-scrim").addEventListener("click",closeArtifacts);
$("#delivery-format").addEventListener("change",deliveryFormatConfig);$("#delivery-backend").addEventListener("change",deliveryFormatConfig); $("#delivery-coverage").addEventListener("change",()=>{deliveryCoverageConfig();renderStoryboard()});
$("#copy-scene-brief").addEventListener("click",copySceneBrief); $("#pick-delivery-asset").addEventListener("click",pickDeliveryAsset);
$("#run-delivery").addEventListener("click",runDelivery); $("#cancel-delivery").addEventListener("click",cancelDelivery); $("#refresh-deliveries").addEventListener("click",renderDeliveries);
$("#approve-delivery-review").addEventListener("click",()=>reviewDelivery(true)); $("#reject-delivery-review").addEventListener("click",()=>reviewDelivery(false));
$("#project-synopsis").addEventListener("input",e=>$("#synopsis-counter").textContent=`${e.target.value.length} / 120000`);
$("#project-form").addEventListener("reset",()=>{++formEpoch;state.files=[];setTimeout(()=>{$("#synopsis-counter").textContent="0 / 120000";$("#file-label").textContent="PDF, DOCX, TXT, MD, or images (max 25 MB each)"},0)});

async function installNativeDrop(){
  const webview=window.__TAURI__?.webview?.getCurrentWebview?.(); if(!webview)return;
  await webview.onDragDropEvent(event=>{
    const zone=$("#dropzone");
    if(event.payload.type==="over"){zone.classList.add("dragging");return}
    zone.classList.remove("dragging");
    if(event.payload.type==="drop"&&$("#screen-project").classList.contains("active")){
      if(mutationPending())return;
      try{setReferenceFiles(event.payload.paths.map(source_path=>({source_path,name:source_path.split(/[\\/]/).at(-1),size:null})))}catch(error){showBanner(String(error),"error")}
    }
  });
}

(async()=>{
  const preview=new URLSearchParams(location.search).get("screen");
  if(!tauriInvoke){if(preview==="pipeline")state.project=DEMO;if(preview==="review")state.project=DEMO_REVIEW;if(preview==="final")state.project=DEMO_FINAL;if(preview)route(preview);showBanner("Browser preview · sample data only · workflow effects and Human decisions are disabled.");return}
  const startup=await Promise.allSettled([loadProjects(),checkRuntime(true),installNativeDrop()]);
  if(state.projects[0]&&!state.project&&projectEpoch===0&&!mutationPending())await openProject(state.projects[0].name);
  const errors=startup.filter(result=>result.status==="rejected").map(result=>String(result.reason));
  if(errors.length)showBanner(`Native setup failed: ${errors.join("; ")}`,"error");
})();
