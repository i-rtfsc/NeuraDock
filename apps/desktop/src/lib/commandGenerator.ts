import type { TokenDto, AccountDto } from '@/types/token';

type AITool = 'claude' | 'codex' | 'gemini';

interface GenerateCommandOptions {
  tool: AITool;
  token: TokenDto;
  account: AccountDto;
  selectedNode: string;
  selectedModel: string;
  isSingleLine: boolean;
}

export function generateCommand({
  tool,
  token,
  account,
  selectedNode,
  selectedModel,
  isSingleLine,
}: GenerateCommandOptions): string {
  const baseUrl = selectedNode || account.provider_domain;
  const apiKey = token.key;

  const commands: Record<AITool, string> = {
    claude: generateClaudeCommand(baseUrl, apiKey, selectedModel, isSingleLine),
    codex: generateCodexCommand(baseUrl, apiKey, selectedModel, isSingleLine),
    gemini: generateGeminiCommand(baseUrl, apiKey, selectedModel, isSingleLine),
  };

  return commands[tool];
}

function generateClaudeCommand(
  baseUrl: string,
  apiKey: string,
  model: string,
  isSingleLine: boolean
): string {
  const modelFlag = model ? ` --model ${model}` : '';
  
  if (isSingleLine) {
    return `export ANTHROPIC_API_KEY="${apiKey}" && export ANTHROPIC_BASE_URL="${baseUrl}" && claude${modelFlag}`;
  }

  return `export ANTHROPIC_API_KEY="${apiKey}"
export ANTHROPIC_BASE_URL="${baseUrl}"
claude${modelFlag}`;
}

function generateCodexCommand(
  baseUrl: string,
  apiKey: string,
  model: string,
  isSingleLine: boolean
): string {
  const modelFlag = model ? ` -m ${model}` : '';
  
  if (isSingleLine) {
    return `export OPENAI_API_KEY="${apiKey}" && export OPENAI_BASE_URL="${baseUrl}" && aider${modelFlag}`;
  }

  return `export OPENAI_API_KEY="${apiKey}"
export OPENAI_BASE_URL="${baseUrl}"
aider${modelFlag}`;
}

function generateGeminiCommand(
  baseUrl: string,
  apiKey: string,
  model: string,
  isSingleLine: boolean
): string {
  const modelFlag = model ? ` --model ${model}` : '';
  
  if (isSingleLine) {
    return `export GEMINI_API_KEY="${apiKey}" && export GEMINI_BASE_URL="${baseUrl}" && gemini${modelFlag}`;
  }

  return `export GEMINI_API_KEY="${apiKey}"
export GEMINI_BASE_URL="${baseUrl}"
gemini${modelFlag}`;
}

export function getToolIcon(tool: AITool): string {
  const icons: Record<AITool, string> = {
    claude: '🤖',
    codex: '💻',
    gemini: '✨',
  };
  return icons[tool];
}

export function getToolName(tool: AITool): string {
  const names: Record<AITool, string> = {
    claude: 'Claude',
    codex: 'Aider (OpenAI)',
    gemini: 'Gemini',
  };
  return names[tool];
}
