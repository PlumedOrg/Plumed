/** What every startup wizard step receives. */
export interface StepProps {
    /** The question is answered (or the step is done): slide to the next one. */
    onNext: () => void;
}
