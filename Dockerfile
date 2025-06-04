FROM public.ecr.aws/lambda/python:3.13

COPY requirements.txt ${LAMBDA_TASK_ROOT}/requirements.txt

RUN pip install -r requirements.txt

COPY service ${LAMBDA_TASK_ROOT}/service

CMD [ "service.handlers.handle_sns_event.lambda_handler" ]
