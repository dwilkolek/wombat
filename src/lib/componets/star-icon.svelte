<script lang="ts">
	export let state: boolean;
	export let size = '1.8em';
	import Icon from 'svelte-icon/Icon.svelte';
	import star from '$lib/images/star-solid.svg?raw';
	let selected = '#fcd703';
	let notSelected = 'rgba(252, 215, 3, 0.5)';
	let transition = state ? notSelected : selected;
	let color = state ? selected : notSelected;
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
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
