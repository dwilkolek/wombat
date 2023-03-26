import {
  derived,
  readable,
  writable,
  type Readable,
  type Writable,
} from "svelte/store";
import { invoke } from "@tauri-apps/api/tauri";

export enum Environment {
  LAB = "LAB",
  DEV = "DEV",
  DEMO = "DEMO",
  PROD = "PROD",
}

export const services: Readable<string[]> = readable([]);
export const dbs: Readable<string[]> = readable([]);

function createState() {
  const env: Writable<Environment> = writable(Environment.DEV);
  const services: Writable<string[]> = writable([]);
  const dbs: Writable<string[]> = writable([]);
  env.subscribe(async (env) => {
    await invoke("set_environment", { env: `${env}` });
    services.set(await invoke("get_services"));
    dbs.set(await invoke("get_rds"));
  });
  return {
    env,
    services,
    dbs,
    selectEnvironment: (newEnv: Environment) => {
      env.set(newEnv);
    },
  };
}

export const state = createState();
