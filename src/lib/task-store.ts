import { writable } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';

type ProxyEventMessage = {
	arn: string;
	port: number;
};
const createTaskStore = () => {
	const runningTasks = writable<ProxyEventMessage[]>([]);
	listen<ProxyEventMessage>('proxy-start', (event) => {
		runningTasks.update((tasks) => {
			return [...tasks, { arn: event.payload.arn, port: event.payload.port }];
		});
	});
	listen<ProxyEventMessage>('proxy-end', (event) => {
		runningTasks.update((tasks) => {
			return tasks.filter((t) => t.arn != event.payload.arn);
		});
	});
	return { subscribe: runningTasks.subscribe };
};
export const taskStore = createTaskStore();
