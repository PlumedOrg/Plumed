import clsx from 'clsx';
import { Check, Copy, X } from 'lucide-react';
import { i18n } from '@/i18n';
import { OneLineText } from '@/components/one-line-text';
import { LINE_HEIGHT_RATIO } from '../streaming-text/streaming-text.helpers';
import { buildResultPanelGradient, RESULT_PANEL_SEGMENT_MASK } from './result-panel.helpers';

interface ResultPanelProps {
    text: string;
    textWidth: number;
    fontSize: number;
    maxLines: number;
    durationSecs: number;
    isPaused: boolean;
    isCopied: boolean;
    onCopy: () => void;
    onPause: () => void;
    onResume: () => void;
    onClose: () => void;
}

export const ResultPanel = ({
    text,
    textWidth,
    fontSize,
    maxLines,
    durationSecs,
    isPaused,
    isCopied,
    onCopy,
    onPause,
    onResume,
    onClose,
}: ResultPanelProps) => {
    return (
        <div
            data-interactive
            onMouseEnter={onPause}
            onMouseLeave={onResume}
            className={clsx(
                'group',
                'relative',
                'overflow-hidden',
                'rounded-t-lg',
                'bg-black',
                'px-2.5',
                'pt-1.5',
                'pb-2'
            )}
            style={{ width: `${textWidth}px` }}
        >
            <button
                type="button"
                aria-label={i18n.t('Copy')}
                onClick={onCopy}
                className="absolute inset-0 z-10 cursor-pointer"
            />

            <button
                type="button"
                aria-label={i18n.t('Close')}
                onClick={onClose}
                className={clsx(
                    'absolute',
                    'top-1',
                    'right-1',
                    'z-20',
                    'hidden',
                    'group-hover:flex',
                    'h-[18px]',
                    'w-[18px]',
                    'cursor-pointer',
                    'items-center',
                    'justify-center',
                    'rounded-full',
                    'bg-black',
                    'text-neutral-300',
                    'transition-all',
                    'duration-150',
                    'ease-out',
                    'hover:scale-110',
                    'hover:bg-neutral-700',
                    'hover:text-white'
                )}
            >
                <X size={10} strokeWidth={2.5} />
            </button>

            {/* Plumed: line breaks as ↵ and an ellipsis past maxLines; Copy gives the whole text. */}
            <div
                className="overflow-hidden font-sans text-white"
                style={{
                    fontSize: `${fontSize}px`,
                    lineHeight: LINE_HEIGHT_RATIO,
                    display: '-webkit-box',
                    WebkitBoxOrient: 'vertical',
                    WebkitLineClamp: maxLines,
                }}
            >
                <OneLineText text={text} />
            </div>

            <div
                className={clsx(
                    'flex h-4 items-center justify-end gap-1 pt-1',
                    isCopied ? 'text-white' : 'text-neutral-600 group-hover:text-white'
                )}
            >
                {isCopied ? <Check size={12} strokeWidth={2.5} /> : <Copy size={12} strokeWidth={2.5} />}
                <span className={clsx('text-[10px] font-normal leading-3', !isCopied && 'hidden group-hover:inline')}>
                    {isCopied ? i18n.t('Copied') : i18n.t('Copy')}
                </span>
            </div>

            <div
                className="absolute bottom-0 left-0 right-0 h-1 bg-neutral-800"
                style={{
                    maskImage: RESULT_PANEL_SEGMENT_MASK,
                    WebkitMaskImage: RESULT_PANEL_SEGMENT_MASK,
                }}
            >
                <div
                    className="h-full w-full"
                    style={{
                        backgroundImage: buildResultPanelGradient(),
                        backgroundSize: `${textWidth}px 100%`,
                        backgroundRepeat: 'no-repeat',
                        animation: `result-panel-countdown ${durationSecs}s linear forwards`,
                        animationPlayState: isPaused ? 'paused' : 'running',
                    }}
                />
            </div>
        </div>
    );
};
