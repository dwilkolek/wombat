<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import type { CustomHeader } from '$lib/types';

	interface Props {
		onRemove?: (name: string) => void;
		onAdd?: (header: CustomHeader) => void;
		added?: boolean;
		disabled: boolean;
		name?: string;
		value?: string;
		encodeBase64?: boolean;
	}

	let {
		onRemove = () => {},
		onAdd = () => {},
		added = false,
		disabled,
		name = $bindable(''),
		value = $bindable(''),
		encodeBase64 = $bindable(false)
	}: Props = $props();
</script>

<form class="flex items-center justify-between gap-2">
	<label class="form-control w-full max-w-xs">
		<input
			{disabled}
			autocomplete="off"
			autocorrect="off"
			autocapitalize="off"
			spellcheck="false"
			type="text"
			placeholder="name"
			bind:value={name}
			class="input input-sm input-bordered w-full max-w-xs"
		/>
	</label>
	<label class="form-control w-full max-w-xs">
		<input
			{disabled}
			autocomplete="off"
			autocorrect="off"
			autocapitalize="off"
			spellcheck="false"
			type="text"
			placeholder="value"
			bind:value
			class="input input-sm input-bordered w-full max-w-xs"
		/>
	</label>
	<div class="form-control">
		<label class="label cursor-pointer">
			<input
				{disabled}
				type="checkbox"
				bind:checked={encodeBase64}
				class="checkbox checkbox-sm checkbox-primary"
			/>

			<span class="label-text pl-1">Encode base64</span>
		</label>
	</div>
	{#if !added}
		<button
			disabled={disabled || name == ''}
			class="btn btn-circle btn-xs btn-success"
			data-umami-event="custom_header_add"
			data-umami-event-uid={$userStore.id}
			onclick={() => {
				onAdd({ name, value, encodeBase64 });
				name = '';
				value = '';
				encodeBase64 = false;
			}}
			aria-label="Add header"
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-4 h-4"
			>
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
			</svg>
		</button>
	{:else}
		<button
			data-umami-event="custom_header_remove"
			data-umami-event-uid={$userStore.id}
			class="btn btn-circle btn-xs btn-error"
			{disabled}
			onclick={(e) => {
				e.preventDefault();
				onRemove(name);
			}}
			aria-label="Remove header"
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-4 h-4"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
				/>
			</svg>
		</button>
	{/if}
</form>
