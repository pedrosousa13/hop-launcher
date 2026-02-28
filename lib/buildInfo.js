import {formatBuildLabel} from './buildLabel.js';

export const BUILD_ID = 'local';
export const BUILD_HASH = '';
export const BUILD_LABEL = formatBuildLabel(BUILD_ID, BUILD_HASH);
