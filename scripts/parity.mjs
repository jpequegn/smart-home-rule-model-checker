import {readFileSync} from 'node:fs';
import {mkdtempSync,writeFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {deepStrictEqual} from 'node:assert';
import init,{analyze} from '../web/pkg/home_rule_wasm.js';
await init({module_or_path:readFileSync(new URL('../web/pkg/home_rule_wasm_bg.wasm',import.meta.url))});
const yaml=readFileSync('examples/alarm.yaml','utf8'),scenario=readFileSync('examples/alarm.json','utf8');
for(const op of ['lint','simulate','verify','minimize','report']){
 const native=spawnSync('target/debug/homecheck',[op,'--rules','examples/alarm.yaml','--scenario','examples/alarm.json'],{encoding:'utf8',maxBuffer:33554432});
 if(![0,1,3].includes(native.status))throw Error(native.stderr);
 deepStrictEqual(JSON.parse(analyze(op,yaml,scenario)),JSON.parse(native.stdout));
 console.log(op+': native/WASM parity');
}
const directory=mkdtempSync(join(tmpdir(),'homecheck-parity-'));
try {
 const corpus=JSON.parse(readFileSync('examples/corpus.json','utf8'));
 for(const c of corpus.cases){
  const yamlPath=join(directory,'rules.yaml'),scenarioPath=join(directory,'scenario.json');
  writeFileSync(yamlPath,c.yaml);writeFileSync(scenarioPath,JSON.stringify(c.scenario));
  for(const op of ['lint','simulate']){
   const native=spawnSync('target/debug/homecheck',[op,'--rules',yamlPath,'--scenario',scenarioPath],{encoding:'utf8',maxBuffer:33554432});
   if(![0,1,3].includes(native.status))throw Error(c.name+': '+native.stderr);
   deepStrictEqual(JSON.parse(analyze(op,c.yaml,JSON.stringify(c.scenario))),JSON.parse(native.stdout),c.name);
  }
 }
 console.log(corpus.cases.length+' corpus cases: native/WASM lint and replay parity');
} finally {rmSync(directory,{recursive:true});}
