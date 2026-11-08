<!-- Action row of a modal: right aligned from sm up; on phones the actions
stack full width with the last one (the main action) on top. The
dialog-actions class lets page styles and tests find the row. -->
<div
  class="dialog-actions mt-6 flex flex-col-reverse gap-3 sm:flex-row sm:justify-end [&>*]:w-full sm:[&>*]:w-auto"
>
  <slot />
</div>
