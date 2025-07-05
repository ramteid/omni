<script lang="ts">
    import { badgeVariants, type Variant } from './index.js'
    import type { HTMLAttributes } from 'svelte/elements'

    type $$Props = HTMLAttributes<HTMLDivElement> & {
        variant?: Variant
    }

    export let variant: Variant = 'default'

    let className: $$Props['class'] = undefined
    export { className as class }
</script>

<div class={badgeVariants({ variant, class: className })} {...$$restProps}>
    <slot />
</div>
