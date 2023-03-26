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

type Entry = {
  service: String;
  service_arn: String;
  dbs: DbInstance[];
};

type Endpoint = {
  address: String;
  port: number;
};

type DbInstance = {
  db_name: String;
  endpoint: Endpoint;
  db_instance_arn: String;
  env: Environment;
  service: String;
};

function createState() {
  const env: Writable<Environment> = writable(Environment.DEV);
  const records: Writable<Entry[]> = writable([]);
  env.subscribe(async (env) => {
    await invoke("set_environment", { env: `${env}` });
    records.set(await invoke("records"));
  });
  return {
    env,
    records,
    selectEnvironment: (newEnv: Environment) => {
      env.set(newEnv);
    },
  };
}

export const state = createState();
