// Mount the retained guide with an isolated permission port, never real TCC.
import { createRoot } from 'react-dom/client';
import { RequiredPermissionsGate } from '../views/RequiredPermissionsGate';
import type { RequiredPermissionsRuntime } from '../application/permissions/runtime';
import '../styles/index.css';
const parameters=new URLSearchParams(location.search);
document.documentElement.dataset.interfaceTheme=parameters.get('theme')??'native';
document.documentElement.dataset.theme=parameters.get('appearance')??'light';
const scenario=parameters.get('scenario')??'initial';
const status={screenRecording:false,accessibility:false};
const runtime:RequiredPermissionsRuntime={
  subscribe(receive){receive({status,error:null});return()=>{};},
  async context(){return{platform:'macos',appPath:'/Applications/SnapLingo.app',needsInstallation:scenario==='installation',canReset:true};},
  async request(){if(scenario==='error'){await new Promise(resolve=>setTimeout(resolve,20));throw new Error('Synthetic request failure');}return status;},
  async reset(){throw new Error('The guide must not reset permissions');},
  async restart(){throw new Error('The guide must not restart');},
  async refresh(){return status;},
};
createRoot(document.getElementById('root')!).render(<RequiredPermissionsGate runtime={runtime} onOpenSettings={()=>{}}><main>Settings fixture</main></RequiredPermissionsGate>);
if(scenario==='attempted'||scenario==='error'){
  const timer=setInterval(()=>{const button=[...document.querySelectorAll('button')].find(button=>button.textContent==='去授权'&&!button.disabled);if(button){clearInterval(timer);button.click();}},10);
}
