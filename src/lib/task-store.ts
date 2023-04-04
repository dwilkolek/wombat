import { invoke } from '@tauri-apps/api';
import { writable } from 'svelte/store';
import { emit, listen } from '@tauri-apps/api/event';

type Task = {
	arn: string;
	type: 'db-proxy';
};
const createTaskStore = () => {
	let runningTasks = writable<Task[]>([]);
	const unlisten = listen<{ arn: string; status: string; type: string }>('db-proxy', (event) => {
		console.log(event);
		runningTasks.update((tasks) => {
			if (event.payload.status == 'START') {
				return [...tasks, { arn: event.payload.arn, type: 'db-proxy' }];
			} else {
				return tasks.filter((t) => t.arn != event.payload.arn);
			}
		});
	});
	return { subscribe: runningTasks.subscribe };
};
export const taskStore = createTaskStore();
