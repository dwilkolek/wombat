import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { AwsEnv } from '$lib/types';

export type ProxyEventMessage = {
	arn: string;
	port: number;
	name: string;
	env: AwsEnv;
	proxy_type: 'ECS' | 'RDS'
};
const createTaskStore = () => {
	const runningTasks = writable<ProxyEventMessage[]>([]);
	listen<ProxyEventMessage>('proxy-start', (event) => {
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
