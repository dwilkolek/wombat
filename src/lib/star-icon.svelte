<script lang="ts">
	export let state: boolean;
	export let size = '2.2em';
	import Icon from 'svelte-icon/Icon.svelte';
	import star from '$lib/images/star-solid.svg?raw';
	let selected = 'hsl(var(--wa))';
	let notSelected = 'hsl(var(--er))';
	let transition = 'hsl(var(--af))';
	let color = state ? selected : notSelected;
</script>

<div
	on:mouseover={() => {
		color = transition;
	}}
	on:mouseout={() => {
		color = state ? selected : notSelected;
	}}
	on:focus={() => {
		color = transition;
	}}
	on:blur={() => {
		color = state ? selected : notSelected;
	}}
>
	<Icon data={star} {size} fill={color} stroke={color} />
</div>
