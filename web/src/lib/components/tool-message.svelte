<script lang="ts">
    import * as Accordion from '$lib/components/ui/accordion'
    import { Search, FileText, TextSearch, ExternalLink } from '@lucide/svelte'
    import type { ToolMessageContent, ToolName } from '$lib/types/message'
    import { cn } from '$lib/utils'
    import {
        getIconFromSearchResult,
        getSourceIconPath,
        getSourceDisplayName,
    } from '$lib/utils/icons'
    import { SourceType } from '$lib/types'

    type Props = {
        message: ToolMessageContent
    }

    const ToolIndicators = {
        search_documents: {
            loading: 'searching',
            loaded: 'searched',
        },
        read_document: {
            loading: 'reading',
            loaded: 'read',
        },
    }

    const ToolInputKey = {
        search_documents: 'query',
        read_document: 'name',
    }

    let { message }: Props = $props()
    const toolName = message.toolUse.name as ToolName
    let statusIndicator = $derived(
        message.toolResult
            ? ToolIndicators[toolName]?.loaded
            : ToolIndicators[toolName]?.loading || 'using',
    )
    const toolInputKey = ToolInputKey[toolName] || 'query'

    let sources = $derived<string[]>(message.toolUse.input?.sources || [])

    let selectedItem = $state<string>()
</script>

{#if toolName === 'read_document'}
    <div
        class={cn(
            'flex cursor-pointer items-center justify-between rounded-md border border-gray-200 px-3 py-3 text-sm hover:no-underline',
        )}>
        <div class="flex w-full items-center justify-between">
            <div class="flex items-center gap-2">
                <TextSearch class="h-5 w-5" />
                <div class="max-w-screen-md truncate text-sm font-normal">
                    {statusIndicator}: {message.toolUse.input[toolInputKey]}
                </div>
            </div>
        </div>
    </div>
{:else}
    <Accordion.Root type="single" bind:value={selectedItem}>
        <Accordion.Item value={message.toolUse.id}>
            <Accordion.Trigger
                class={cn(
                    'flex cursor-pointer items-center justify-between border border-gray-200 px-3 py-3 text-sm hover:no-underline',
                    selectedItem === message.toolUse.id && 'bg-card rounded-b-none border-b-0',
                )}>
                <div class="flex w-full items-center justify-between">
                    <div class="flex items-center gap-2">
                        {#if sources.length > 0}
                            <div class="flex items-center gap-1">
                                {#each sources as source}
                                    {#if getSourceIconPath(source)}
                                        <img
                                            src={getSourceIconPath(source)}
                                            alt={getSourceDisplayName(source as SourceType) ||
                                                source}
                                            title={getSourceDisplayName(source as SourceType) ||
                                                source}
                                            class="!m-0 h-4 w-4" />
                                    {/if}
                                {/each}
                            </div>
                        {:else}
                            <Search class="h-4 w-4" />
                        {/if}
                        <div class="max-w-screen-md truncate text-sm font-normal">
                            {#if sources.length > 0}
                                {statusIndicator}
                                {sources
                                    .map((s) => getSourceDisplayName(s as SourceType) || s)
                                    .join(', ')}: {message.toolUse.input[toolInputKey]}
                            {:else}
                                {statusIndicator}: {message.toolUse.input[toolInputKey]}
                            {/if}
                        </div>
                    </div>
                    {#if message.toolResult}
                        <div class="text-muted-foreground text-xs">
                            {message.toolResult.content.length} results
                        </div>
                    {/if}
                </div>
            </Accordion.Trigger>
            {#if message.toolResult && message.toolResult.content.length > 0}
                <Accordion.Content
                    class="bg-card max-h-48 overflow-y-auto rounded-b-md border border-t-0 border-gray-200">
                    <div class="px-4 py-2">
                        <div class="flex flex-col gap-2">
                            {#each message.toolResult.content as result}
                                <div class="flex items-center gap-2">
                                    {#if getIconFromSearchResult(result.source)}
                                        <img
                                            src={getIconFromSearchResult(result.source)}
                                            alt=""
                                            class="!m-0 h-4 w-4 flex-shrink-0" />
                                    {:else}
                                        <FileText
                                            class="text-muted-foreground h-4 w-4 flex-shrink-0" />
                                    {/if}
                                    <a
                                        href={result.source.split('#')[0]}
                                        target="_blank"
                                        class="block max-w-screen-sm overflow-hidden font-normal text-ellipsis whitespace-nowrap no-underline hover:underline">
                                        {result.title}
                                    </a>
                                </div>
                            {/each}
                        </div>
                    </div>
                </Accordion.Content>
            {/if}
        </Accordion.Item>
    </Accordion.Root>
{/if}
