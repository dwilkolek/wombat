import { derived, readable, writable, type Readable, type Writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/tauri';

export enum Environment {
	PLAY = 'PLAY',
	LAB = 'LAB',
	DEV = 'DEV',
	DEMO = 'DEMO',
	PROD = 'PROD',
	UNKNOWN = 'UNKNOWN'
}

export type EcsService = {
	name: string;
	service_arn: string;
	env: Environment;
};
type Entry = {
	service: String;
	service_arn: String;
	dbs: DbInstance[];
};

type Endpoint = {
	address: String;
	port: number;
};

export type DbInstance = {
	db_name: String;
	endpoint: Endpoint;
	db_instance_arn: String;
	env: Environment;
	service: String;
};

function createState() {
	const env: Writable<Environment> = writable(Environment.DEV);
	const records: Writable<Entry[]> = writable([]);
	const profile: Writable<string> = writable();
	const error: Writable<string | undefined> = writable();
	env.subscribe(async (env) => {
		await invoke('set_environment', { env: `${env}` });
		records.set(await invoke('records'));
	});
	profile.subscribe(async () => {
		records.set([]);
	});

	return {
		env,
		records,
		profile,
		error,
		start: async (withProfile: string) => {
			profile.set(withProfile);
			error.set(undefined);
			try {
				records.set(await invoke('login', { profile: withProfile }));
			} catch (e) {
				error.set(`${e}`);
			}
		},
		selectEnvironment: (newEnv: Environment) => {
			env.set(newEnv);
		}
	};
}

export const state = createState();
