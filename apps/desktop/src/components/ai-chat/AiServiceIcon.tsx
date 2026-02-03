import { cn } from '@/lib/utils';
import { Bot } from 'lucide-react';

interface AiServiceIconProps {
  icon: string | null;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

// Only include verified brand SVGs - use generic icon for others
// ChatGPT icon is the official OpenAI logo
const ChatGPTIcon = ({ className }: { className?: string }) => (
  <svg viewBox="0 0 24 24" className={className} fill="currentColor">
    <path d="M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.8956zm16.0993 3.8558L12.602 8.3829l2.0201-1.1638a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.1408 1.6465 4.4708 4.4708 0 0 1 .5765 3.0234zm-12.645 4.1441l-2.02-1.164a.0757.0757 0 0 1-.038-.0519V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.5046-2.6067-1.4998Z"/>
  </svg>
);

// Claude icon - Anthropic's logo
const ClaudeIcon = ({ className }: { className?: string }) => (
  <svg viewBox="0 0 24 24" className={className} fill="currentColor">
    <path d="M4.709 15.955l4.72-2.647.08-.23-.08-.128H8.78l-.08.209-2.527 1.4-.08.048-.08.289.08.289.08.048.536.289-.209.023zm8.67-6.108l.185-.209.787-.913-.185-.209h-.324l-.648.746-.08.096-.08.337.08.08.08.072h.185zm-1.576 8.627l.08-.209-.08-.128h-.648l-.08.209.08.289.08.048 2.287 1.255.08.048.08-.096.185-.386-.08-.096-.787-.434-1.117-.5zm7.804-5.195l-.787.913.185.209h.324l.648-.746.08-.096.08-.337-.08-.08-.08-.072h-.185l-.185.209zM12 2C6.477 2 2 6.477 2 12s4.477 10 10 10 10-4.477 10-10S17.523 2 12 2zm5.373 14.32l-2.287-1.255-.08-.048-.08.096-.185.386.08.096.787.434 1.117.5-.08.209.08.128h.648l.08-.209-.08-.289-.08-.048-.536-.289.08.289zm-6.664-1.279l-.787.913.185.209h.324l.648-.746.08-.096.08-.337-.08-.08-.08-.072h-.185l-.185.209zm1.576-8.627l-.08.209.08.128h.648l.08-.209-.08-.289-.08-.048-2.287-1.255-.08-.048-.08.096-.185.386.08.096.787.434 1.117.5zm-7.804 5.195l.787-.913-.185-.209h-.324l-.648.746-.08.096-.08.337.08.08.08.072h.185l.185-.209zm14.228.346l-4.72 2.647-.08.23.08.128h.648l.08-.209 2.527-1.4.08-.048.08-.289-.08-.289-.08-.048-.536-.289.08.578z"/>
  </svg>
);

// Gemini icon - Google's star logo
const GeminiIcon = ({ className }: { className?: string }) => (
  <svg viewBox="0 0 24 24" className={className} fill="currentColor">
    <path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8z"/>
    <path d="M12 7a5 5 0 1 0 5 5 5 5 0 0 0-5-5zm0 8a3 3 0 1 1 3-3 3 3 0 0 1-3 3z"/>
  </svg>
);

// Map only verified icons - others will use default
const iconMap: Record<string, React.FC<{ className?: string }>> = {
  chatgpt: ChatGPTIcon,
  openai: ChatGPTIcon,
  claude: ClaudeIcon,
  anthropic: ClaudeIcon,
  gemini: GeminiIcon,
  google: GeminiIcon,
};

const sizeClasses = {
  sm: 'w-4 h-4',
  md: 'w-6 h-6',
  lg: 'w-8 h-8',
};

export function AiServiceIcon({ icon, className, size = 'md' }: AiServiceIconProps) {
  const IconComponent = icon ? iconMap[icon.toLowerCase()] : null;
  
  // Use lucide Bot icon as fallback for services without verified brand icons
  if (!IconComponent) {
    return <Bot className={cn(sizeClasses[size], className)} />;
  }

  return <IconComponent className={cn(sizeClasses[size], className)} />;
}
