/* eslint-disable @typescript-eslint/no-explicit-any */
export const load: any = ({ params }: { params: { app: string } }) => {
	return {
		app: params.app
	};
};

export interface AppPage {
	app: string;
}
