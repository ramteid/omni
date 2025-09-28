<script lang="ts">
    import { Button } from '$lib/components/ui/button/index.js'
    import { Input } from '$lib/components/ui/input/index.js'
    import { Send, Sparkles, Search } from '@lucide/svelte'
    import { marked } from 'marked'
    import type { PageProps } from './$types.js'
    import type { MessageStreamEvent } from '@anthropic-ai/sdk/resources/messages.js'
    import type { MessageParam } from '@anthropic-ai/sdk/resources/messages.js'
    import { onMount } from 'svelte'
    import type {
        TextBlockParam,
        SearchResultBlockParam,
    } from '@anthropic-ai/sdk/resources/messages'
    import * as Accordion from '$lib/components/ui/accordion'

    type ToolComponentData = {
        id: number
        role: 'tool'
        toolUse: {
            id: string
            name: string
            input: any
        }
        toolResult?: {
            toolUseId: string // Same as toolUse.id
            content: {
                title: string
                source: string
            }[]
        }
    }

    type TextComponentData = {
        id: number
        role: 'user' | 'assistant'
        content: string
    }

    type MessageComponentData = TextComponentData | ToolComponentData

    let { data }: PageProps = $props()

    let inputValue = $state('')
    let isLoading = $state(false)
    let answer = $state('')
    let formattedAnswer = $derived(marked.parse(answer))
    let error = $state<string | null>(null)

    let processedMessages = $derived(processMessages(data.messages.map((m) => m.message)))

    function renderUserMessage(message: MessageParam) {
        if (typeof message.content === 'string') {
            return marked.parse(message.content)
        } else if (Array.isArray(message.content)) {
            const textContent = message.content
                .filter((block) => block.type === 'text')
                .map((block) => block.text)
                .join('\n\n')
            return marked.parse(textContent)
        }
        return `<span class="text-red-600">[Something went wrong rendering this message.]</span>`
    }

    function processMessages(messages: MessageParam[]): MessageComponentData[] {
        const components: MessageComponentData[] = []
        const toolCallMap = new Map<string, ToolComponentData>()

        for (let i = 0; i < messages.length; i++) {
            const message = messages[i]
            if (isUserMessage(message)) {
                // User messages are expected to contain only text blocks
                if (typeof message.content === 'string') {
                    components.push({
                        id: components.length,
                        role: 'user',
                        content: message.content,
                    })
                } else if (Array.isArray(message.content)) {
                    const textBlocks = message.content.filter((block) => block.type === 'text')
                    for (const block of textBlocks) {
                        components.push({
                            id: components.length,
                            role: 'user',
                            content: block.text,
                        })
                    }
                }
            } else {
                // Here we handle both assistant messages (with possible tool uses) and also user messages that contain tool results
                const contentBlocks = Array.isArray(message.content)
                    ? message.content
                    : [{ type: 'text', text: message.content } as TextBlockParam]

                for (const block of contentBlocks) {
                    if (block.type === 'text') {
                        components.push({
                            id: components.length,
                            role: 'assistant',
                            content: block.text,
                        })
                    } else {
                        // Tool use or result
                        if (block.type === 'tool_use') {
                            // Tool use always comes first, so we create the corresponding output block
                            const toolComponentData: ToolComponentData = {
                                id: components.length,
                                role: 'tool',
                                toolUse: {
                                    id: block.id,
                                    name: block.name,
                                    input: block.input,
                                },
                            }

                            components.push(toolComponentData)
                            toolCallMap.set(block.id, toolComponentData)
                        } else if (block.type === 'tool_result') {
                            const toolUseId = block.tool_use_id
                            const toolComponentData = toolCallMap.get(toolUseId)
                            const searchResults = Array.isArray(block.content)
                                ? (block.content.filter(
                                      (b) => b.type === 'search_result',
                                  ) as SearchResultBlockParam[])
                                : []
                            toolComponentData!.toolResult = {
                                toolUseId,
                                content: searchResults.map((r) => ({
                                    title: r.title,
                                    source: r.source,
                                })),
                            }
                        }
                    }
                }
            }
        }

        // Combine consecutive text blocks from the same role
        const combinedComponents: MessageComponentData[] = []

        for (const component of components) {
            const last = combinedComponents[combinedComponents.length - 1]
            if (
                last &&
                (last.role === 'user' || last.role === 'assistant') &&
                last.role === component.role
            ) {
                last.content += '\n' + component.content
            } else {
                combinedComponents.push(component)
            }
        }

        return combinedComponents
    }

    function isUserMessage(message: MessageParam) {
        // Tool results, even though found in messages with role 'user', are shown as assistant messages
        const toolResults = Array.isArray(message.content)
            ? message.content.some((block) => block.type === 'tool_result')
            : false
        return message.role === 'user' && !toolResults
    }

    // Start streaming when we have a query
    onMount(() => {
        streamAIResponse(data.chat.id)
    })

    function streamAIResponse(chatId: string) {
        isLoading = true
        answer = ''
        error = null

        const eventSource = new EventSource(`/api/chat/${chatId}/stream`, { withCredentials: true })

        let accumulatedText = ''
        let streamCompleted = false

        eventSource.addEventListener('message', (event) => {
            try {
                const data: MessageStreamEvent = JSON.parse(event.data)
                if (data.type === 'content_block_start') {
                    if (
                        data.content_block.type === 'tool_use' &&
                        data.content_block.name === 'search_documents'
                    ) {
                        accumulatedText += '\n\n🔍 Searching documents...\n\n'
                        answer = accumulatedText
                        isLoading = false
                    }
                } else if (data.type === 'content_block_delta') {
                    if (data.delta.type === 'text_delta' && data.delta.text) {
                        accumulatedText += data.delta.text
                        answer = accumulatedText
                        isLoading = false
                    } else if (data.delta.type === 'input_json_delta') {
                        // Parse partial JSON to show search query if possible
                    }
                }
            } catch (err) {
                console.warn('Failed to parse SSE data:', event.data, err)
            }
        })

        eventSource.addEventListener('end_of_stream', () => {
            streamCompleted = true
            isLoading = false
            eventSource.close()
        })

        eventSource.addEventListener('error', (event) => {
            error = 'Failed to generate response. Please try again.'
            isLoading = false
            eventSource.close()
        })
    }

    function handleSubmit() {
        if (inputValue.trim()) {
        }
    }

    function handleKeyPress(event: KeyboardEvent) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault()
            handleSubmit()
        }
    }
</script>

<svelte:head>
    <title>Omni Chat</title>
</svelte:head>

<div class="flex h-[calc(100vh-4rem)] flex-col">
    <!-- Chat Container -->
    <div class="flex-1 overflow-y-auto px-4 py-6">
        <div class="mx-auto max-w-4xl">
            <!-- Existing Messages -->
            {#each processedMessages as message (message.id)}
                {#if message.role === 'user'}
                    <!-- User Message -->
                    <div class="mb-6 flex justify-end">
                        <div class="text-foreground max-w-[80%] rounded-2xl bg-gray-100 px-4 py-2">
                            {@html marked.parse(message.content)}
                        </div>
                    </div>
                {:else if message.role === 'assistant'}
                    <!-- Assistant Message -->
                    <div class="mb-6">
                        <div class="flex items-start gap-3">
                            <div
                                class="flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-purple-500 to-pink-500">
                                <Sparkles class="h-5 w-5 text-white" />
                            </div>
                            <div class="flex-1">
                                <div class="prose max-w-none">
                                    {@html marked.parse(message.content)}
                                </div>
                            </div>
                        </div>
                    </div>
                {:else if message.role === 'tool'}
                    <!-- Tool Use and Result -->
                    <div class="mb-6">
                        <Accordion.Root type="multiple">
                            <Accordion.Item value={`tool-${message.id}`}>
                                <Accordion.Trigger
                                    class="flex cursor-pointer items-center justify-between border border-gray-200 px-4 text-sm hover:no-underline">
                                    <div class="flex items-center gap-2">
                                        <Search class="h-4 w-4" />
                                        <div class="font-normal">{message.toolUse.input.query}</div>
                                    </div>
                                </Accordion.Trigger>
                                {#if message.toolResult && message.toolResult.content.length > 0}
                                    <Accordion.Content
                                        class="bg-card max-h-48 overflow-y-auto border border-gray-200">
                                        <div class="px-4 py-2">
                                            <div class="flex flex-col gap-2">
                                                {#each message.toolResult.content as result}
                                                    <div class="">
                                                        <a
                                                            href={result.source}
                                                            target="_blank"
                                                            class="hover:underline">
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
                    </div>
                {/if}
            {/each}

            <!-- Streaming AI Response (only show if streaming is active) -->
            {#if isLoading || answer || error}
                <div class="mb-6">
                    <div class="flex items-start gap-3">
                        <div
                            class="flex h-8 w-8 items-center justify-center rounded-full bg-gradient-to-br from-purple-500 to-pink-500">
                            <Sparkles class="h-5 w-5 text-white" />
                        </div>
                        <div class="flex-1">
                            {#if isLoading && !answer}
                                <div class="text-gray-500">Thinking...</div>
                            {:else if error}
                                <div class="text-red-600">
                                    {error}
                                </div>
                            {:else if formattedAnswer}
                                <div class="prose prose-sm max-w-none">
                                    {@html formattedAnswer}
                                </div>
                            {/if}
                        </div>
                    </div>
                </div>
            {/if}
        </div>
    </div>

    <!-- Input Area -->
    <div class="border-t bg-white px-4 py-4">
        <div class="mx-auto max-w-4xl">
            <div class="flex items-center gap-2">
                <Input
                    type="text"
                    bind:value={inputValue}
                    placeholder="Ask a question..."
                    class="flex-1"
                    onkeypress={handleKeyPress}
                    disabled={isLoading} />
                <Button
                    onclick={handleSubmit}
                    disabled={isLoading || !inputValue.trim()}
                    size="icon">
                    <Send class="h-4 w-4" />
                </Button>
            </div>
        </div>
    </div>
</div>
