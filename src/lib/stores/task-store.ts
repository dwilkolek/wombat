import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv, JepsenConfig } from '$lib/types';

export type ProxyEventMessage = {
	arn: string;
	port: number;
	status: string,
	name: string;
	env: AwsEnv;
	proxy_type: 'ECS' | 'RDS';
	jepsen_config: JepsenConfig | null | undefined;
};
const createTaskStore = () => {
	const runningTasks = writable<ProxyEventMessage[]>([]);
	listen<ProxyEventMessage>('proxy-start', (event) => {
		console.log('proxy-start', (event))
		runningTasks.update((tasks) => {
			return [...tasks.filter(t => t.arn != event.payload.arn), { ...event.payload }];
		});
	});
	listen<ProxyEventMessage>('proxy-starting', (event) => {
		console.log('proxy-start', (event))
		runningTasks.update((tasks) => {
			return [...tasks, { ...event.payload }];
		});
	});
	listen<ProxyEventMessage>('proxy-end', (event) => {
		console.log('proxy-end', (event))
		runningTasks.update((tasks) => {
			return tasks.filter((t) => t.arn != event.payload.arn);
		});
	});
	return { subscribe: runningTasks.subscribe };
};
export const taskStore = createTaskStore();
