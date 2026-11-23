import { Fragment } from 'react';

/**
 * Plumed fork (written with Claude Code): a text with its line breaks shown as
 * a ↵ sign, for previews where the text runs on (the caller clamps it with an
 * ellipsis). "Examen :\n\n- Calme" shows as "Examen : ↵ - Calme": consecutive
 * breaks count as one.
 */
export const OneLineText = ({ text }: { text: string }) => (
    <>
        {text.split(/\s*\n[\s\n]*/).map((line, index) => (
            <Fragment key={index}>
                {index > 0 && (
                    <span className="mx-1 opacity-50" aria-label="↵">
                        ↵
                    </span>
                )}
                {line}
            </Fragment>
        ))}
    </>
);
