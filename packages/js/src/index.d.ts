export type Phase="starting"|"navigating"|"observing"|"waiting_for_model"|"acting"|"verifying"|"settling"|"finished";
export interface ConnectOptions{socketPath?:string;sessionId?:string;cdpUrl?:string;framework?:string;workload?:string;browserVersion?:string}
export interface PhaseOptions{expectedWaitMs?:number;criticalLiveConnection?:boolean;timeoutMs?:number}
export class BaeldAgent{static connect(options?:ConnectOptions):Promise<BaeldAgent>;phase(name:Phase,options?:PhaseOptions):Promise<Record<string,unknown>>;waitingForModel(expectedWaitMs:number,options?:PhaseOptions):Promise<Record<string,unknown>>;acting(options?:PhaseOptions):Promise<Record<string,unknown>>;verify(options?:PhaseOptions):Promise<Record<string,unknown>>;finish(options?:PhaseOptions):Promise<Record<string,unknown>>;session():{id:string;cdpUrl?:string;filterProfile?:string;mode?:string};close():void}
export function connectPlaywright(playwright:any,options?:ConnectOptions):Promise<{browser:any;agent:BaeldAgent}>;
export function connectStagehand(api:{Stagehand:any;localBrowser:any},options?:ConnectOptions&{logging?:any;stagehandOptions?:Record<string,unknown>}):Promise<{stagehand:any;browser:any;agent:BaeldAgent}>;
export function installFiltering(page:any,profile?:"safe"|"balanced"|"text"|"visual"):Promise<void>;
export function withModelWait<T>(agent:BaeldAgent,expectedWaitMs:number,operation:()=>Promise<T>,options?:PhaseOptions):Promise<T>;
