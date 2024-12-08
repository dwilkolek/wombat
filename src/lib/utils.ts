export function* getFromList<T>(list: T[]): Generator<T> {
	for (let i = 0; i < list.length; i++) {
		yield list[i];
	}
}
